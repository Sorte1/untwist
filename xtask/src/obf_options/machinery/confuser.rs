use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::parse_program;

use super::shapes::{ident_name, member_name};
use super::Found;

const BASE91_TABLE_LEN: usize = 91;
const INTEGRITY_SENSITIVITY: &str = "|\\{|\\}|\\(|\\)|";
const DISPATCHER_PARAMS: usize = 4;
const MIN_PACKED_CODE: usize = 200;
const GLOBAL_NAMES: [&str; 3] = ["globalThis", "global", "window"];

pub const CONFUSER: [&str; 16] = [
    "jsc_string_decoders",
    "jsc_string_getters",
    "jsc_buffer_to_string",
    "jsc_global_getters",
    "jsc_global_switches",
    "jsc_dispatchers",
    "jsc_calculators",
    "jsc_literal_arrays",
    "jsc_opaque_predicates",
    "jsc_cff_dispatch",
    "jsc_masked_params",
    "jsc_flat_functions",
    "jsc_integrity",
    "jsc_self_defending",
    "jsc_tamper_checks",
    "jsc_packed",
];

#[derive(Default)]
struct Declared {
    empty_functions: BTreeSet<String>,
    literal_arrays: BTreeSet<String>,
    references: BTreeMap<String, usize>,
    indexed: BTreeMap<String, usize>,
}

impl Declared {
    fn only_indexed(&self, name: &str) -> bool {
        self.literal_arrays.contains(name)
            && self.indexed.get(name).copied().unwrap_or(0) > 0
            && self.indexed.get(name) == self.references.get(name)
    }
}

impl<'a> Visit<'a> for Declared {
    fn visit_identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        *self.references.entry(id.name.to_string()).or_default() += 1;
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        if let (Some(name), Expression::NumericLiteral(_)) =
            (ident_name(&member.object), &member.expression)
        {
            *self.indexed.entry(name.to_string()).or_default() += 1;
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        let empty = func.params.items.is_empty()
            && func.params.rest.is_none()
            && func.body.as_ref().is_some_and(|b| b.statements.is_empty());
        if let (true, Some(id), true) = (func.is_declaration(), &func.id, empty) {
            self.empty_functions.insert(id.name.to_string());
        }
        walk::walk_function(self, func, flags);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(Expression::ArrayExpression(array))) =
            (&declarator.id, &declarator.init)
        {
            if !array.elements.is_empty() && array.elements.iter().all(is_literal_element) {
                self.literal_arrays.insert(id.name.to_string());
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }
}

fn is_literal_element(element: &ArrayExpressionElement) -> bool {
    matches!(
        element,
        ArrayExpressionElement::StringLiteral(_)
            | ArrayExpressionElement::NumericLiteral(_)
            | ArrayExpressionElement::BooleanLiteral(_)
            | ArrayExpressionElement::NullLiteral(_)
    )
}

struct Scan<'d> {
    declared: &'d Declared,
    found: Found,
}

fn is_base91_table(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let distinct: BTreeSet<char> = chars.iter().copied().collect();
    chars.len() == BASE91_TABLE_LEN
        && distinct.len() == BASE91_TABLE_LEN
        && chars.iter().all(|c| c.is_ascii_graphic())
}

fn single_return<'b, 'a>(body: &'b FunctionBody<'a>) -> Option<&'b Expression<'a>> {
    match body.statements.as_slice() {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

fn is_slice_getter(func: &Function) -> bool {
    if func.params.items.len() != 2 {
        return false;
    }
    let Some(Expression::CallExpression(outer)) = func
        .body
        .as_ref()
        .and_then(|b| single_return(b))
        .map(|e| e.without_parentheses())
    else {
        return false;
    };
    let [Argument::CallExpression(inner)] = outer.arguments.as_slice() else {
        return false;
    };
    matches!(member_name(&inner.callee), Some((_, "slice"))) && inner.arguments.len() == 2
}

fn returns_global(element: &ArrayExpressionElement) -> bool {
    let func = match element {
        ArrayExpressionElement::FunctionExpression(f) => f,
        _ => return false,
    };
    let Some(expr) = func.body.as_ref().and_then(|b| single_return(b)) else {
        return false;
    };
    match expr.without_parentheses() {
        Expression::Identifier(id) => GLOBAL_NAMES.contains(&id.name.as_str()),
        Expression::CallExpression(call) => matches!(
            call.callee.without_parentheses(),
            Expression::NewExpression(n) if ident_name(&n.callee) == Some("Function")
        ),
        _ => false,
    }
}

fn case_returns<'b, 'a>(case: &'b SwitchCase<'a>) -> Option<&'b Expression<'a>> {
    match case.consequent.as_slice() {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

fn is_global_switch(switch: &SwitchStatement) -> bool {
    switch.cases.len() >= 2
        && switch.cases.iter().all(|case| {
            case.test
                .as_ref()
                .is_some_and(|t| matches!(t, Expression::StringLiteral(_)))
                && case_returns(case).is_some_and(|e| {
                    matches!(e.without_parentheses(), Expression::ComputedMemberExpression(m)
                        if matches!(m.expression, Expression::StringLiteral(_)) && ident_name(&m.object).is_some())
                })
        })
}

fn is_calculator(switch: &SwitchStatement) -> bool {
    !switch.cases.is_empty()
        && switch.cases.iter().all(|case| {
            case.test
                .as_ref()
                .is_some_and(|t| matches!(t, Expression::StringLiteral(_)))
                && case_returns(case).is_some_and(|e| {
                    matches!(e.without_parentheses(), Expression::BinaryExpression(b)
                        if ident_name(&b.left).is_some() && ident_name(&b.right).is_some())
                })
        })
}

fn sums_states(expr: &Expression) -> bool {
    matches!(expr.without_parentheses(), Expression::CallExpression(call)
        if call.arguments.len() == 1 && matches!(call.arguments[0], Argument::Identifier(_)) && ident_name(&call.callee).is_some())
}

fn is_dispatcher(func: &Function) -> bool {
    func.params.items.len() == DISPATCHER_PARAMS
        && matches!(
            &func.params.items[DISPATCHER_PARAMS - 1].initializer.as_deref(),
            Some(Expression::ObjectExpression(o)) if o.properties.is_empty()
        )
}

fn is_masked(func: &Function) -> bool {
    func.params.items.is_empty() && func.params.rest.is_some() && {
        let mut finder = NumericIndex {
            name: rest_name(func),
            found: false,
        };
        if let Some(body) = &func.body {
            finder.visit_function_body(body);
        }
        finder.found
    }
}

fn rest_name(func: &Function) -> String {
    match func.params.rest.as_ref().map(|r| &r.rest.argument) {
        Some(BindingPattern::BindingIdentifier(id)) => id.name.to_string(),
        _ => String::new(),
    }
}

struct NumericIndex {
    name: String,
    found: bool,
}

impl<'a> Visit<'a> for NumericIndex {
    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        if ident_name(&member.object) == Some(self.name.as_str())
            && matches!(member.expression, Expression::NumericLiteral(_))
        {
            self.found = true;
        }
        walk::walk_computed_member_expression(self, member);
    }
}

fn is_flat(func: &Function) -> bool {
    matches!(
        func.params.items.as_slice(),
        [first, second]
            if matches!(first.pattern, BindingPattern::BindingIdentifier(_))
                && matches!(second.pattern, BindingPattern::ArrayPattern(_))
    )
}

fn is_native_code(text: &str) -> bool {
    text == "{ [native code] }"
}

fn is_packed(call: &CallExpression) -> bool {
    let (constructor, arguments) = match call.callee.without_parentheses() {
        Expression::CallExpression(inner) => (&inner.callee, &inner.arguments),
        Expression::NewExpression(inner) => (&inner.callee, &inner.arguments),
        _ => return false,
    };
    ident_name(constructor) == Some("Function")
        && arguments
            .iter()
            .any(|a| matches!(a, Argument::StringLiteral(s) if s.value.len() >= MIN_PACKED_CODE))
}

impl<'a> Visit<'a> for Scan<'_> {
    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        let Some(value) = lit.value.as_str() else {
            return;
        };
        if is_base91_table(value) {
            self.found.add("jsc_string_decoders", 1);
        }
        if value == "TextDecoder" {
            self.found.add("jsc_buffer_to_string", 1);
        }
        if value.contains(INTEGRITY_SENSITIVITY) {
            self.found.add("jsc_integrity", 1);
        }
        if is_native_code(value) {
            self.found.add("jsc_tamper_checks", 1);
        }
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        if is_slice_getter(func) {
            self.found.add("jsc_string_getters", 1);
        }
        if is_dispatcher(func) {
            self.found.add("jsc_dispatchers", 1);
        }
        if is_masked(func) {
            self.found.add("jsc_masked_params", 1);
        }
        if is_flat(func) {
            self.found.add("jsc_flat_functions", 1);
        }
        walk::walk_function(self, func, flags);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let masked = arrow.params.items.is_empty() && arrow.params.rest.is_some() && {
            let name = match arrow.params.rest.as_ref().map(|r| &r.rest.argument) {
                Some(BindingPattern::BindingIdentifier(id)) => id.name.to_string(),
                _ => String::new(),
            };
            let mut finder = NumericIndex { name, found: false };
            finder.visit_arrow_function_expression(arrow);
            finder.found
        };
        if masked {
            self.found.add("jsc_masked_params", 1);
        }
        walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_array_expression(&mut self, array: &ArrayExpression<'a>) {
        if array.elements.len() >= 3 && array.elements.iter().all(returns_global) {
            self.found.add("jsc_global_getters", 1);
        }
        walk::walk_array_expression(self, array);
    }

    fn visit_switch_statement(&mut self, switch: &SwitchStatement<'a>) {
        if is_global_switch(switch) {
            self.found.add("jsc_global_switches", 1);
        }
        if is_calculator(switch) {
            self.found.add("jsc_calculators", 1);
        }
        walk::walk_switch_statement(self, switch);
    }

    fn visit_while_statement(&mut self, stmt: &WhileStatement<'a>) {
        let sums = matches!(&stmt.test, Expression::BinaryExpression(b)
            if b.operator == BinaryOperator::StrictInequality && sums_states(&b.left));
        if sums {
            self.found.add("jsc_cff_dispatch", 1);
        }
        walk::walk_while_statement(self, stmt);
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression<'a>) {
        let predicate = binary.operator == BinaryOperator::In
            && matches!(binary.left, Expression::StringLiteral(_))
            && ident_name(&binary.right).is_some_and(|n| self.declared.empty_functions.contains(n));
        if predicate {
            self.found.add("jsc_opaque_predicates", 1);
        }
        walk::walk_binary_expression(self, binary);
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        let indexed = matches!(member.expression, Expression::NumericLiteral(_))
            && ident_name(&member.object).is_some_and(|n| self.declared.only_indexed(n));
        if indexed {
            self.found.add("jsc_literal_arrays", 1);
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_packed(call) {
            self.found.add("jsc_packed", 1);
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, new: &NewExpression<'a>) {
        let newline = ident_name(&new.callee) == Some("RegExp")
            && matches!(new.arguments.as_slice(), [Argument::StringLiteral(s)] if s.value.as_str() == Some("\n"));
        if newline {
            self.found.add("jsc_self_defending", 1);
        }
        walk::walk_new_expression(self, new);
    }
}

pub fn measure_into(source: &str, found: &mut Found) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut declared = Declared::default();
    declared.visit_program(&parsed.program);
    let mut scan = Scan {
        declared: &declared,
        found: Found::default(),
    };
    scan.visit_program(&parsed.program);
    for (class, count) in scan.found.counts {
        found.add(class, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(source: &str) -> Vec<&'static str> {
        let mut found = Found::default();
        measure_into(source, &mut found);
        found.present(&CONFUSER)
    }

    #[test]
    fn each_shape_is_named() {
        let table: String = (b'!'..=b'~')
            .filter(|b| *b != b'\'' && *b != b'\\' && *b != b'-')
            .take(91)
            .map(char::from)
            .collect();
        let rows = [
            (format!("var t = \"{}\";", table.replace('"', "\\\"")), "jsc_string_decoders"),
            ("function g(s, l) { return d(a.slice(s, s + l)); }".to_string(), "jsc_string_getters"),
            ("var d = o[\"TextDecoder\"];".to_string(), "jsc_buffer_to_string"),
            ("var a = [function () { return globalThis; }, function () { return global; }, function () { return new Function(\"return this\")(); }];".to_string(), "jsc_global_getters"),
            ("function g(m) { switch (m) { case \"a\": return o[\"console\"]; case \"b\": return o[\"x\"]; } }".to_string(), "jsc_global_switches"),
            ("function d(n, f, r, l = {}) {}".to_string(), "jsc_dispatchers"),
            ("function c(o, a, b) { switch (o) { case \"k\": return a + b; } }".to_string(), "jsc_calculators"),
            ("const d = [\"log\", 1]; console[d[0]](d[1]);".to_string(), "jsc_literal_arrays"),
            ("function e() {} if (!(\"k\" in e)) {}".to_string(), "jsc_opaque_predicates"),
            ("while (s(v) !== 9) {}".to_string(), "jsc_cff_dispatch"),
            ("function f(...m) { return m[0]; }".to_string(), "jsc_masked_params"),
            ("function f(o, [a, b]) {}".to_string(), "jsc_flat_functions"),
            ("var r = new RegExp(\" |\\\\n|;|,|\\\\{|\\\\}|\\\\(|\\\\)|\\\\.\", \"g\");".to_string(), "jsc_integrity"),
            ("var r = new RegExp(\"\\n\");".to_string(), "jsc_self_defending"),
            ("if (s.indexOf(\"{ [native code] }\") < 0) {}".to_string(), "jsc_tamper_checks"),
            (format!("Function(\"a\", \"{}\")(1);", "x;".repeat(120)), "jsc_packed"),
        ];
        for (source, class) in rows {
            assert_eq!(classes(&source), [class], "{source}");
        }
    }

    #[test]
    fn plain_code_measures_clean() {
        let source = "const words = [\"a\", \"b\"]; console.log(words[0], words.length); function f(a, b = {}) { return a; } switch (x) { case 1: break; }";
        assert!(classes(source).is_empty());
    }
}
