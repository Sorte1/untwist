use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;

const BASE64_ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/";

const RC4_STATE_SIZE: f64 = 256.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Encodings {
    pub base64: bool,
    pub rc4: bool,
}

impl Encodings {
    pub fn names(self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.rc4 {
            names.push("rc4");
        }
        if self.base64 {
            names.push("base64");
        }
        names
    }
}

pub fn decoder_encodings(program: &Program<'_>, decoders: &[Span]) -> Encodings {
    let mut finder = DecoderFinder {
        decoders,
        found: Encodings::default(),
    };
    finder.visit_program(program);
    finder.found
}

struct DecoderFinder<'d> {
    decoders: &'d [Span],
    found: Encodings,
}

impl<'a> VisitJs<'a> for DecoderFinder<'_> {
    fn visit_statement(&mut self, statement: &Statement<'a>) {
        if !self.decoders.contains(&statement.span()) {
            walk_js::walk_statement(self, statement);
            return;
        }
        let mut scan = MarkerScan::default();
        scan.visit_statement(statement);
        let rc4 = scan.found.rc4 && scan.found.base64;
        self.found.rc4 |= rc4;
        self.found.base64 |= scan.found.base64 && !rc4;
    }
}

#[derive(Default)]
struct MarkerScan {
    found: Encodings,
}

impl<'a> VisitJs<'a> for MarkerScan {
    fn visit_string_literal(&mut self, literal: &StringLiteral<'a>) {
        if literal
            .value
            .as_str()
            .is_some_and(|value| value.contains(BASE64_ALPHABET))
        {
            self.found.base64 = true;
        }
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression<'a>) {
        if binary.operator == BinaryOperator::Remainder
            && matches!(binary.right.without_parentheses(), Expression::NumericLiteral(n) if n.value == RC4_STATE_SIZE)
        {
            self.found.rc4 = true;
        }
        walk_js::walk_binary_expression(self, binary);
    }
}
