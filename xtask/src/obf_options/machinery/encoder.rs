use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BinaryExpression, CallExpression, Expression, NewExpression, ObjectExpression,
    ObjectPropertyKind, PropertyKey, UnaryExpression,
};
use oxc_ast_visit::{walk, Visit};
use untwist_core::parse::parse_program;

use super::shapes::member_name;
use super::Found;

const JJENCODE_KEY: &str = "___";
const AA_MARKS: [&str; 2] = ["ﾟωﾟﾉ", "ﾟДﾟ"];
const RUNNERS: [&str; 2] = ["eval", "Function"];

pub const ENCODER: [&str; 4] = [
    "enc_coercions",
    "enc_jjencode",
    "enc_aaencode",
    "enc_code_runners",
];

pub fn measure_into(source: &str, found: &mut Found) {
    found.add(
        "enc_aaencode",
        AA_MARKS
            .iter()
            .map(|mark| source.matches(mark).count())
            .sum(),
    );
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut runners = Runners::default();
    runners.visit_program(&parsed.program);
    found.add("enc_coercions", runners.coercions);
    found.add("enc_jjencode", runners.jjencode);
    found.add("enc_code_runners", runners.count);
}

fn is_empty_array(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ArrayExpression(array) => array.elements.is_empty(),
        Expression::UnaryExpression(unary) => is_empty_array(&unary.argument),
        _ => false,
    }
}

#[derive(Default)]
struct Runners {
    count: usize,
    coercions: usize,
    jjencode: usize,
}

fn is_runner(callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => RUNNERS.contains(&id.name.as_str()),
        Expression::SequenceExpression(seq) => seq.expressions.last().is_some_and(is_runner),
        other => member_name(other)
            .is_some_and(|(_, name)| name == "constructor" || RUNNERS.contains(&name)),
    }
}

impl<'a> Visit<'a> for Runners {
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if is_empty_array(&it.left) || is_empty_array(&it.right) {
            self.coercions += 1;
        }
        walk::walk_binary_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if is_empty_array(&it.argument) {
            self.coercions += 1;
        }
        walk::walk_unary_expression(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        let keyed = it.properties.iter().any(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => matches!(
                &property.key,
                PropertyKey::StaticIdentifier(id) if id.name == JJENCODE_KEY
            ),
            ObjectPropertyKind::SpreadProperty(_) => false,
        });
        if keyed {
            self.jjencode += 1;
        }
        walk::walk_object_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_runner(&it.callee) {
            self.count += 1;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if is_runner(&it.callee) {
            self.count += 1;
        }
        walk::walk_new_expression(self, it);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(source: &str) -> Vec<&'static str> {
        let mut found = Found::default();
        measure_into(source, &mut found);
        found.present(&ENCODER)
    }

    #[test]
    fn each_encoder_shape_is_named() {
        let rows = [
            ("x(![] + []);".to_string(), "enc_coercions"),
            ("$={___:1};".to_string(), "enc_jjencode"),
            ("ﾟωﾟﾉ= 1;".to_string(), "enc_aaencode"),
            ("eval(s);".to_string(), "enc_code_runners"),
        ];
        for (source, class) in rows {
            assert_eq!(classes(&source), [class], "{source}");
        }
    }

    #[test]
    fn plain_code_measures_clean() {
        let source =
            "var a = [1, 2]; console.log(a.map(function (x) { return x + 1; }).join(\"-\"), !a);";
        assert!(classes(source).is_empty());
    }
}
