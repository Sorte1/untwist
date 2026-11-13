use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::js_text::js_text;

pub const COMMONJS_GLOBAL: &str = "commonjsGlobal";
pub const DEFAULT_EXPORT: &str = "getDefaultExportFromCjs";
pub const AUGMENTED_NAMESPACE: &str = "getAugmentedNamespace";
pub const COMMONJS_REQUIRE: &str = "commonjsRequire";
pub const COMMONJS_MIN: &str = "__commonJSMin";

const GLOBAL_NAMES: [&str; 4] = ["globalThis", "window", "global", "self"];
const DYNAMIC_REQUIRE: &str = "Could not dynamically require";
const MAX_HELPER_STATEMENTS: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Helper {
    pub symbol: SymbolId,
    pub name: &'static str,
    pub scope: Option<ScopeId>,
}

#[derive(Default)]
struct Words {
    keys: FxHashSet<String>,
    texts: Vec<String>,
    statements: usize,
    nested: bool,
}

impl<'a> VisitJs<'a> for Words {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.keys.insert(it.property.name.to_string());
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        let text = js_text(it.value).to_string();
        self.keys.insert(text.clone());
        self.texts.push(text);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        if let Some(cooked) = it.value.cooked {
            self.texts.push(js_text(cooked).to_string());
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        self.statements += 1;
        walk_js::walk_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.nested = true;
        walk_js::walk_function(self, it, flags);
    }
}

fn words(function: &Function<'_>) -> Option<Words> {
    let body = function.body.as_deref()?;
    let mut words = Words::default();
    for statement in &body.statements {
        words.visit_statement(statement);
    }
    Some(words)
}

fn has(words: &Words, keys: &[&str]) -> bool {
    keys.iter().all(|key| words.keys.contains(*key))
}

fn one_param(function: &Function<'_>) -> bool {
    function.params.items.len() == 1 && function.params.rest.is_none()
}

fn function_helper(function: &Function<'_>) -> Option<&'static str> {
    if function.r#async || function.generator {
        return None;
    }
    let words = words(function)?;
    if words
        .texts
        .iter()
        .any(|text| text.starts_with(DYNAMIC_REQUIRE))
    {
        return Some(COMMONJS_REQUIRE);
    }
    if words.statements > MAX_HELPER_STATEMENTS {
        return None;
    }
    let interop = has(&words, &["__esModule", "hasOwnProperty", "call", "default"]);
    if interop && one_param(function) && !words.nested && !words.keys.contains("defineProperty") {
        return Some(DEFAULT_EXPORT);
    }
    let augmented = has(
        &words,
        &[
            "__esModule",
            "defineProperty",
            "keys",
            "forEach",
            "getOwnPropertyDescriptor",
        ],
    );
    (augmented && one_param(function)).then_some(AUGMENTED_NAMESPACE)
}

fn typeof_global(test: &Expression<'_>) -> Option<&'static str> {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return None;
    };
    if !matches!(
        binary.operator,
        BinaryOperator::Inequality | BinaryOperator::StrictInequality
    ) {
        return None;
    }
    let operand = match (&binary.left, &binary.right) {
        (Expression::UnaryExpression(unary), Expression::StringLiteral(text))
        | (Expression::StringLiteral(text), Expression::UnaryExpression(unary))
            if unary.operator == UnaryOperator::Typeof && js_text(text.value) == "undefined" =>
        {
            &unary.argument
        }
        _ => return None,
    };
    match operand.without_parentheses() {
        Expression::Identifier(id) => GLOBAL_NAMES
            .iter()
            .find(|name| **name == id.name.as_str())
            .copied(),
        _ => None,
    }
}

fn is_global_chain(expression: &Expression<'_>) -> bool {
    let mut current = expression.without_parentheses();
    let mut seen = 0usize;
    while let Expression::ConditionalExpression(conditional) = current {
        let Some(name) = typeof_global(&conditional.test) else {
            return false;
        };
        let same = matches!(conditional.consequent.without_parentheses(), Expression::Identifier(id) if id.name.as_str() == name);
        if !same {
            return false;
        }
        seen += 1;
        current = conditional.alternate.without_parentheses();
    }
    let empty =
        matches!(current, Expression::ObjectExpression(object) if object.properties.is_empty());
    empty && seen >= 3
}

fn plain_identifiers(params: &FormalParameters<'_>, count: usize) -> bool {
    params.rest.is_none()
        && params.items.len() == count
        && params
            .items
            .iter()
            .all(|param| matches!(param.pattern, BindingPattern::BindingIdentifier(_)))
}

fn arrow_value<'k, 'a>(arrow: &'k ArrowFunctionExpression<'a>) -> Option<&'k Expression<'a>> {
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(_) => None,
        other => other.as_expression(),
    }
}

fn is_commonjs_min(arrow: &ArrowFunctionExpression<'_>) -> bool {
    if !plain_identifiers(&arrow.params, 2) {
        return false;
    }
    let Some(Expression::ArrowFunctionExpression(inner)) =
        arrow_value(arrow).map(Expression::without_parentheses)
    else {
        return false;
    };
    if !plain_identifiers(&inner.params, 0) {
        return false;
    }
    match &inner.body {
        ArrowFunctionBody::FunctionBody(body) => guarded_statements(&body.statements),
        other => other
            .as_expression()
            .is_some_and(|value| guarded_sequence(value.without_parentheses())),
    }
}

fn returns_exports(expression: &Expression<'_>) -> bool {
    matches!(expression.without_parentheses(), Expression::StaticMemberExpression(member) if member.property.name == "exports")
}

fn guarded_sequence(expression: &Expression<'_>) -> bool {
    let Expression::SequenceExpression(sequence) = expression else {
        return false;
    };
    let [guard, exports] = &sequence.expressions[..] else {
        return false;
    };
    let guarded = matches!(guard.without_parentheses(), Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::Or);
    guarded && returns_exports(exports)
}

fn guarded_statements(statements: &[Statement<'_>]) -> bool {
    let [Statement::IfStatement(guard), Statement::ReturnStatement(last)] = statements else {
        return false;
    };
    let negated = matches!(guard.test.without_parentheses(), Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot);
    negated && guard.alternate.is_none() && last.argument.as_ref().is_some_and(returns_exports)
}

#[derive(Default)]
pub struct HelperFinder {
    pub found: Vec<Helper>,
}

impl<'a> VisitJs<'a> for HelperFinder {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() {
            let symbol = it.id.as_ref().and_then(|id| id.symbol_id.get());
            if let (Some(symbol), Some(name)) = (symbol, function_helper(it)) {
                self.found.push(Helper {
                    symbol,
                    name,
                    scope: it.scope_id.get(),
                });
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let found = match init.without_parentheses() {
                Expression::ArrowFunctionExpression(arrow) if is_commonjs_min(arrow) => {
                    Some((COMMONJS_MIN, arrow.scope_id.get()))
                }
                other if is_global_chain(other) => Some((COMMONJS_GLOBAL, None)),
                _ => None,
            };
            if let (Some((name, scope)), Some(symbol)) = (found, id.symbol_id.get()) {
                self.found.push(Helper {
                    symbol,
                    name,
                    scope,
                });
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
