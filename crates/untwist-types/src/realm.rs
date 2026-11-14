use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{Scoping, SemanticBuilder};
use untwist_core::semantic::builtins::{scan, Builtins};

use crate::builtins::RELIANCE;
use crate::erase;

const DOCUMENT_WRITERS: [&str; 3] = ["write", "writeln", "createContextualFragment"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Realm {
    pub builtins: bool,
    pub promises: bool,
}

impl Realm {
    pub const UNKNOWN: Realm = Realm {
        builtins: false,
        promises: false,
    };

    pub fn meet(self, other: Realm) -> Realm {
        Realm {
            builtins: self.builtins && other.builtins,
            promises: self.promises && other.promises,
        }
    }
}

struct Loaders {
    found: bool,
}

fn local_module(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../")
}

fn literal_text<'e>(expression: &'e Expression<'_>) -> Option<&'e str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

impl<'a> Visit<'a> for Loaders {
    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.found |= literal_text(&it.source).is_none_or(|text| !local_module(text));
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            let name = member.static_property_name().and_then(|name| name.as_str());
            if name.is_some_and(|name| DOCUMENT_WRITERS.contains(&name)) {
                self.found = true;
            }
            if name == Some("createElement") {
                let tag = it
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .and_then(literal_text);
                self.found |= tag.is_none_or(|tag| tag.eq_ignore_ascii_case("script"));
            }
        }
        walk::walk_call_expression(self, it);
    }
}

pub(crate) fn loads_code(program: &Program<'_>) -> bool {
    let mut loaders = Loaders { found: false };
    loaders.visit_program(program);
    loaders.found
}

pub(crate) fn of_program(program: &Program<'_>, scoping: &Scoping, module: bool) -> Realm {
    let scanned = scan(program, scoping, module, false, &RELIANCE);
    let loads = loads_code(program);
    Realm {
        builtins: scanned.verdict == Builtins::Intact && !loads,
        promises: !scanned.promise_written && !loads,
    }
}

fn of_text(code: &str) -> Realm {
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
    of_program(program, &scoping, module)
}

pub fn realm(texts: &[&str]) -> Realm {
    texts.iter().fold(
        Realm {
            builtins: true,
            promises: true,
        },
        |held, text| held.meet(of_text(text)),
    )
}
