use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{Scoping, SemanticBuilder};
use untwist_core::semantic::builtins::{scan, Builtins};

use crate::builtins::RELIANCE;
use crate::erase;

const QUIET_GLOBALS: &[&str] = &[
    "Math",
    "JSON",
    "Number",
    "String",
    "Boolean",
    "Array",
    "Object",
    "Date",
    "Symbol",
    "Promise",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "BigInt",
    "Reflect",
    "Proxy",
    "ArrayBuffer",
    "DataView",
    "Int8Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "Int16Array",
    "Uint16Array",
    "Int32Array",
    "Uint32Array",
    "Float32Array",
    "Float64Array",
    "BigInt64Array",
    "BigUint64Array",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURIComponent",
    "encodeURI",
    "decodeURIComponent",
    "decodeURI",
    "escape",
    "unescape",
    "undefined",
    "NaN",
    "Infinity",
    "console",
    "exports",
];

const ERROR_GLOBALS: &[&str] = &[
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "URIError",
    "EvalError",
    "AggregateError",
];

const SHAPED_GLOBALS: &[&str] = &["module", "require", "arguments"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Realm {
    pub builtins: bool,
    pub promises: bool,
    pub closed_calls: bool,
    pub self_contained: bool,
}

impl Realm {
    pub const UNKNOWN: Realm = Realm {
        builtins: false,
        promises: false,
        closed_calls: false,
        self_contained: false,
    };

    pub fn meet(self, other: Realm) -> Realm {
        Realm {
            builtins: self.builtins && other.builtins,
            promises: self.promises && other.promises,
            closed_calls: self.closed_calls && other.closed_calls,
            self_contained: self.self_contained && other.self_contained,
        }
    }
}

struct Loaders<'p, 's> {
    found: bool,
    allowed: Vec<u32>,
    place: Place<'p>,
    scoping: &'s Scoping,
    functions: usize,
}

#[derive(Clone, Copy)]
pub struct Place<'p> {
    pub dir: &'p str,
    pub known: &'p [String],
}

const RESOLVED_SUFFIXES: [&str; 6] = ["", ".js", ".mjs", ".cjs", ".jsx", "/index.js"];

pub fn normalized(dir: &str, specifier: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in specifier.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

impl Place<'_> {
    fn holds(&self, specifier: &str) -> bool {
        if !(specifier.starts_with("./") || specifier.starts_with("../")) {
            return false;
        }
        let Some(base) = normalized(self.dir, specifier) else {
            return false;
        };
        RESOLVED_SUFFIXES.iter().any(|suffix| {
            self.known
                .iter()
                .any(|known| *known == format!("{base}{suffix}"))
        })
    }
}

fn literal_text<'e>(expression: &'e Expression<'_>) -> Option<&'e str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

impl Loaders<'_, '_> {
    fn free(&self, ident: &IdentifierReference<'_>) -> bool {
        ident
            .reference_id
            .get()
            .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }

    fn allow(&mut self, expression: &Expression<'_>, names: &[&str]) {
        if let Expression::Identifier(ident) = expression.without_parentheses() {
            if names.contains(&ident.name.as_str()) {
                self.allowed.push(ident.span.start);
            }
        }
    }

    fn source(&mut self, source: &StringLiteral<'_>) {
        let place = self.place;
        self.found |= source.value.as_str().is_none_or(|text| !place.holds(text));
    }
}

impl<'a> Visit<'a> for Loaders<'_, '_> {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.source(&it.source);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.source(&it.source);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.source(&it.source);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        let place = self.place;
        self.found |= literal_text(&it.source).is_none_or(|text| !place.holds(text));
        walk::walk_import_expression(self, it);
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found = true;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = it.callee.without_parentheses() {
            if callee.name == "require" && self.free(callee) {
                let local = it.arguments.len() == 1
                    && it
                        .arguments
                        .first()
                        .and_then(Argument::as_expression)
                        .and_then(literal_text)
                        .is_some_and(|text| self.place.holds(text));
                if local {
                    self.allowed.push(callee.span.start);
                }
            }
        }
        self.allow(&it.callee, ERROR_GLOBALS);
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.allow(&it.callee, ERROR_GLOBALS);
        walk::walk_new_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator == oxc_syntax::operator::BinaryOperator::Instanceof {
            self.allow(&it.right, ERROR_GLOBALS);
        }
        walk::walk_binary_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(heritage) = &it.heritage {
            self.allow(&heritage.expression, ERROR_GLOBALS);
        }
        walk::walk_class(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == "exports" {
            self.allow(&it.object, &["module"]);
        }
        walk::walk_static_member_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        self.functions += 1;
        walk::walk_function(self, it, flags);
        self.functions -= 1;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if !self.free(it) || QUIET_GLOBALS.contains(&name) {
            return;
        }
        let shaped = match name {
            "arguments" => self.functions > 0,
            _ if SHAPED_GLOBALS.contains(&name) || ERROR_GLOBALS.contains(&name) => {
                self.allowed.contains(&it.span.start)
            }
            _ => false,
        };
        self.found |= !shaped;
    }
}

pub(crate) fn loads_code(program: &Program<'_>, scoping: &Scoping, place: Place<'_>) -> bool {
    let mut loaders = Loaders {
        found: false,
        allowed: Vec::new(),
        place,
        scoping,
        functions: 0,
    };
    loaders.visit_program(program);
    loaders.found
}

fn declares_a_global_name(program: &Program<'_>, scoping: &Scoping, module: bool) -> bool {
    if module || untwist_core::semantic::has_module_syntax(program) {
        return false;
    }
    let root = scoping.root_scope_id();
    scoping.get_bindings(root).keys().any(|name| {
        let name = name.as_str();
        QUIET_GLOBALS.contains(&name)
            || ERROR_GLOBALS.contains(&name)
            || SHAPED_GLOBALS.contains(&name)
            || crate::builtins::RELIANCE.roots.contains(&name)
    })
}

fn has_with(scoping: &Scoping) -> bool {
    scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .contains(oxc_syntax::scope::ScopeFlags::With)
    })
}

pub(crate) fn of_program(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    place: Place<'_>,
) -> Realm {
    let scanned = scan(program, scoping, module, false, &RELIANCE);
    let loads = loads_code(program, scoping, place);
    let declares = declares_a_global_name(program, scoping, module);
    Realm {
        builtins: scanned.verdict == Builtins::Intact && !loads && !declares,
        promises: !scanned.promise_written && !loads && !declares,
        closed_calls: !crate::facts::frames_reachable(program, scoping) && !has_with(scoping),
        self_contained: !loads,
    }
}

fn of_text(code: &str, place: Place<'_>) -> Realm {
    let alloc = Allocator::default();
    let Some((program, source_type)) = erase::parse_js(&alloc, code) else {
        return Realm::UNKNOWN;
    };
    let program = alloc.alloc(program);
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let module = untwist_core::semantic::has_module_syntax(program) || source_type.is_module();
    of_program(program, &scoping, module, place)
}

pub fn realm(texts: &[&str]) -> Realm {
    let placed: Vec<(&str, &str)> = texts.iter().map(|text| (*text, "")).collect();
    realm_in(&placed, &[])
}

pub fn realm_in(texts: &[(&str, &str)], known: &[String]) -> Realm {
    texts.iter().fold(
        Realm {
            builtins: true,
            promises: true,
            closed_calls: true,
            self_contained: true,
        },
        |held, (text, dir)| held.meet(of_text(text, Place { dir, known })),
    )
}
