use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use untwist_core::parse::{has_error, parse_program};

const MIN_ARRAY_STRINGS: usize = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Residue {
    pub decoder_calls: usize,
    pub string_arrays: usize,
    pub rotators: usize,
}

impl Residue {
    pub fn plus(self, other: Residue) -> Residue {
        Residue {
            decoder_calls: self.decoder_calls + other.decoder_calls,
            string_arrays: self.string_arrays + other.string_arrays,
            rotators: self.rotators + other.rotators,
        }
    }

    pub fn at_most(self, other: Residue) -> bool {
        self.decoder_calls <= other.decoder_calls
            && self.string_arrays <= other.string_arrays
            && self.rotators <= other.rotators
    }

    pub fn is_clean(self) -> bool {
        self.string_arrays == 0 && self.rotators == 0
    }

    pub fn render(self) -> String {
        format!(
            "calls={} arrays={} rotators={}",
            self.decoder_calls, self.string_arrays, self.rotators
        )
    }
}

impl Measured {
    pub fn render(self) -> String {
        if self.parses {
            self.residue.render()
        } else {
            format!("{} UNPARSABLE", self.residue.render())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measured {
    pub residue: Residue,
    pub parses: bool,
}

pub fn measure(source: &str) -> Measured {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut counter = Counter::default();
    counter.visit_program(&parsed.program);
    Measured {
        residue: counter.residue,
        parses: !has_error(&parsed.diagnostics),
    }
}

#[derive(Default)]
struct Counter {
    residue: Residue,
}

fn is_number(expr: &Expression) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.argument.without_parentheses(),
                Expression::NumericLiteral(_)
            )
        }
        _ => false,
    }
}

fn is_decoder_call(call: &CallExpression) -> bool {
    if !matches!(call.callee.without_parentheses(), Expression::Identifier(_)) {
        return false;
    }
    let mut args = call.arguments.iter().filter_map(Argument::as_expression);
    let first = args.next().is_some_and(is_number);
    let rest = args.all(|arg| matches!(arg.without_parentheses(), Expression::StringLiteral(_)));
    first && rest && call.arguments.len() <= 2
}

fn member_name<'a>(expr: &'a Expression<'a>) -> Option<(&'a Expression<'a>, &'a str)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(key) => Some((&member.object, key.value.as_str()?)),
            _ => None,
        },
        _ => None,
    }
}

fn ident_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn is_rotation(call: &CallExpression) -> bool {
    let Some((target, "push")) = member_name(&call.callee) else {
        return false;
    };
    let [Argument::CallExpression(inner)] = call.arguments.as_slice() else {
        return false;
    };
    let Some((source, "shift")) = member_name(&inner.callee) else {
        return false;
    };
    ident_name(target).is_some() && ident_name(target) == ident_name(source)
}

fn is_string_array(array: &ArrayExpression) -> bool {
    array.elements.len() >= MIN_ARRAY_STRINGS
        && array
            .elements
            .iter()
            .all(|element| matches!(element, ArrayExpressionElement::StringLiteral(_)))
}

impl<'a> Visit<'a> for Counter {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_decoder_call(call) {
            self.residue.decoder_calls += 1;
        }
        if is_rotation(call) {
            self.residue.rotators += 1;
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_array_expression(&mut self, array: &ArrayExpression<'a>) {
        if is_string_array(array) {
            self.residue.string_arrays += 1;
        }
        walk::walk_array_expression(self, array);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_calls_rotators_and_arrays_are_counted() {
        let m = measure(
            "var a=['a','b','c','d','e','f','g','h','i','j'];(function(){a.push(a.shift());a['push'](a['shift']());})();\
             f(0x1a);g(12,'k3y');h(1,2);x.y(3);z(-4);",
        );
        assert!(m.parses);
        assert_eq!(
            m.residue,
            Residue {
                decoder_calls: 3,
                string_arrays: 1,
                rotators: 2
            }
        );
    }

    #[test]
    fn a_short_or_mixed_array_is_not_a_string_array() {
        let m = measure("var a=['a','b'];var b=['a','b','c','d','e','f','g','h','i',1];");
        assert_eq!(m.residue.string_arrays, 0);
    }

    #[test]
    fn broken_text_does_not_parse() {
        assert!(!measure("function (").parses);
        assert!(measure("export const a = 1;").parses);
    }
}
