use oxc_ast::ast::{Program, Statement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::GetSpan;

#[derive(Default)]
struct Spans(Vec<(u32, u32)>);

impl<'a> VisitJs<'a> for Spans {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let span = it.span();
        self.0.push((span.start, span.end));
        walk_js::walk_statement(self, it);
    }
}

pub fn statement_spans(program: &Program<'_>) -> Vec<(u32, u32)> {
    let mut spans = Spans::default();
    spans.visit_program(program);
    spans.0
}

#[cfg(test)]
mod tests {
    use super::statement_spans;
    use crate::parse::parse_program;
    use oxc_allocator::Allocator;

    #[test]
    fn statements_are_listed_outer_first_in_source_order() {
        let source = "var a = 1;\nfunction f() { if (a) { return a; } }\nf();";
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        let texts: Vec<&str> = statement_spans(&parsed.program)
            .into_iter()
            .map(|(start, end)| &source[start as usize..end as usize])
            .collect();
        assert_eq!(
            texts,
            [
                "var a = 1;",
                "function f() { if (a) { return a; } }",
                "if (a) { return a; }",
                "{ return a; }",
                "return a;",
                "f();"
            ]
        );
    }
}
