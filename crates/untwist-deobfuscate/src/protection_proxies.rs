use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;

use crate::bindings::reference_symbol;

const PROTO_KEY: &str = "__proto__";

#[derive(Clone)]
pub(crate) enum Proxy {
    Text(String),
    Call(usize),
    Binary(BinaryOperator),
}

#[derive(Default)]
pub(crate) struct Proxies {
    objects: FxHashMap<SymbolId, FxHashMap<String, Proxy>>,
}

impl Proxies {
    pub(crate) fn of(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut scan = Scan {
            scoping,
            objects: FxHashMap::default(),
            reads: FxHashMap::default(),
            spoiled: FxHashSet::default(),
        };
        scan.visit_program(program);
        let Scan {
            mut objects,
            reads,
            spoiled,
            ..
        } = scan;
        objects.retain(|symbol, _| {
            let references = scoping.get_resolved_reference_ids(*symbol).len();
            !spoiled.contains(symbol)
                && !scoping.symbol_is_mutated(*symbol)
                && scoping.symbol_redeclarations(*symbol).is_empty()
                && reads.get(symbol).copied().unwrap_or(0) == references
        });
        Self { objects }
    }

    pub(crate) fn member<'k>(
        &'k self,
        expr: &Expression<'_>,
        scoping: &Scoping,
    ) -> Option<&'k Proxy> {
        let (object, key) = member_parts(expr)?;
        let symbol = reference_symbol(object.without_parentheses(), scoping)?;
        self.objects.get(&symbol)?.get(key.as_ref())
    }
}

fn member_parts<'k>(
    expr: &'k Expression<'_>,
) -> Option<(&'k Expression<'k>, std::borrow::Cow<'k, str>)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) if !member.optional => Some((
            &member.object,
            std::borrow::Cow::Borrowed(member.property.name.as_str()),
        )),
        Expression::ComputedMemberExpression(member) if !member.optional => {
            match &member.expression {
                Expression::StringLiteral(key) => Some((&member.object, js_text(key.value))),
                _ => None,
            }
        }
        _ => None,
    }
}

fn simple_params(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .map(|param| match (&param.pattern, &param.initializer) {
            (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn is_param(expr: &Expression<'_>, param: SymbolId, scoping: &Scoping) -> bool {
    reference_symbol(expr.without_parentheses(), scoping) == Some(param)
}

fn function_proxy(function: &Function<'_>, scoping: &Scoping) -> Option<Proxy> {
    if function.r#async || function.generator || function.id.is_some() {
        return None;
    }
    let params = simple_params(function)?;
    let body = function.body.as_ref()?;
    let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
        return None;
    };
    if !body.directives.is_empty() {
        return None;
    }
    match returned.argument.as_ref()?.without_parentheses() {
        Expression::CallExpression(call) if !call.optional => {
            let (callee, rest) = params.split_first()?;
            let forwards = call.arguments.len() == rest.len()
                && call.arguments.iter().zip(rest).all(|(argument, param)| {
                    argument
                        .as_expression()
                        .is_some_and(|argument| is_param(argument, *param, scoping))
                });
            (is_param(&call.callee, *callee, scoping) && forwards)
                .then_some(Proxy::Call(rest.len()))
        }
        Expression::BinaryExpression(binary) => {
            let [left, right] = params.as_slice() else {
                return None;
            };
            (is_param(&binary.left, *left, scoping) && is_param(&binary.right, *right, scoping))
                .then_some(Proxy::Binary(binary.operator))
        }
        _ => None,
    }
}

fn property_proxy(property: &ObjectPropertyKind<'_>, scoping: &Scoping) -> Option<(String, Proxy)> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if property.kind != PropertyKind::Init || property.method || property.shorthand {
        return None;
    }
    let key = match &property.key {
        PropertyKey::StaticIdentifier(name) if !property.computed => name.name.to_string(),
        PropertyKey::StringLiteral(text) => js_text(text.value).into_owned(),
        _ => return None,
    };
    if key == PROTO_KEY {
        return None;
    }
    let proxy = match property.value.without_parentheses() {
        Expression::StringLiteral(text) => Proxy::Text(js_text(text.value).into_owned()),
        Expression::FunctionExpression(function) => function_proxy(function, scoping)?,
        _ => return None,
    };
    Some((key, proxy))
}

fn object_proxies(
    object: &ObjectExpression<'_>,
    scoping: &Scoping,
) -> Option<FxHashMap<String, Proxy>> {
    let mut found = FxHashMap::default();
    for property in &object.properties {
        let (key, proxy) = property_proxy(property, scoping)?;
        if found.insert(key, proxy).is_some() {
            return None;
        }
    }
    Some(found)
}

struct Scan<'s> {
    scoping: &'s Scoping,
    objects: FxHashMap<SymbolId, FxHashMap<String, Proxy>>,
    reads: FxHashMap<SymbolId, usize>,
    spoiled: FxHashSet<SymbolId>,
}

impl Scan<'_> {
    fn object_of(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let (object, _) = member_parts(expr)?;
        reference_symbol(object.without_parentheses(), self.scoping)
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let declared = match (
            &it.id,
            it.init.as_ref().map(Expression::without_parentheses),
        ) {
            (BindingPattern::BindingIdentifier(id), Some(Expression::ObjectExpression(object))) => {
                id.symbol_id.get().zip(object_proxies(object, self.scoping))
            }
            _ => None,
        };
        if let Some((symbol, proxies)) = declared {
            self.objects.insert(symbol, proxies);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Some(symbol) = self.object_of(it) {
            *self.reads.entry(symbol).or_default() += 1;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.spoiled.extend(self.object_of(&it.argument));
        }
        walk_js::walk_unary_expression(self, it);
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use untwist_core::parse::{parse_with, SourceType};
    use untwist_core::semantic::build_scoping;

    use super::*;
    use crate::protection_shape::{is_protection_callback, Roles};

    const CONTROLLER: &str = "var jR = (function () { var jT = true; return function (jU, jV) { var jW = jT ? function () { if (jV) { var jY = jV.apply(jU, arguments); jV = null; return jY; } } : function () {}; jT = false; return jW; }; }());";

    const PROXIED: &str = r#"var jP = { MDSrY: "return /\" + this + \"/", oERss: "^([^ ]+( +[^ ]+)+)+[^ ]}", PaqcT: function (jT) { return jT(); } };
var jQ = jR(this, function () { var jT = function () { return !jT.constructor(jP.MDSrY)().constructor(jP.oERss).test(jQ); }; return jP.PaqcT(jT); });"#;

    fn declarator<'k, 'a>(program: &'k Program<'a>, name: &str) -> &'k VariableDeclarator<'a> {
        program
            .body
            .iter()
            .find_map(|stmt| match stmt {
                Statement::VariableDeclaration(declaration) => declaration
                    .declarations
                    .iter()
                    .find(|it| matches!(&it.id, BindingPattern::BindingIdentifier(id) if id.name == name)),
                _ => None,
            })
            .unwrap()
    }

    fn symbol(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
        match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        }
    }

    fn matches_a_template(source: &str, proxied: bool) -> bool {
        let alloc = Allocator::default();
        let text = format!("{CONTROLLER}\n{source}");
        let parsed = parse_with(&alloc, &text, SourceType::cjs());
        let scoping = build_scoping(&parsed.program).scoping;
        let proxies = Proxies::of(&parsed.program, &scoping);
        let protections = FxHashSet::default();
        let registration = declarator(&parsed.program, "jQ");
        let Some(Expression::CallExpression(call)) = &registration.init else {
            panic!("no registration");
        };
        let roles = Roles {
            own: symbol(registration),
            controller: symbol(declarator(&parsed.program, "jR")),
            protections: Some(&protections),
            proxies: proxied.then_some(&proxies),
            arrows: false,
        };
        let callback = call.arguments[1].as_expression().unwrap();
        is_protection_callback(callback, roles, &scoping)
    }

    #[test]
    fn a_self_defending_callback_read_through_a_constant_control_flow_object_matches() {
        assert!(matches_a_template(PROXIED, true));
        assert!(!matches_a_template(PROXIED, false));
    }

    #[test]
    fn a_control_flow_object_the_program_writes_or_deletes_from_is_not_read_through() {
        for change in ["jP.MDSrY = \"x\";", "delete jP.oERss;", "var jZ = jP;"] {
            let source = format!("{PROXIED}\n{change}");
            assert!(!matches_a_template(&source, true), "{change}");
        }
    }

    #[test]
    fn a_proxy_that_does_more_than_call_or_compare_is_not_read_through() {
        let source = PROXIED.replace("return jT();", "return jT(), 1;");
        assert!(!matches_a_template(&source, true));
    }
}
