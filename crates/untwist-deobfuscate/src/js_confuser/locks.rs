use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{is_foreign_global, may_call_eval};

use super::names::{names, Names};
use super::refs::{inside, scan, unpatched_outside, Refs};
use super::templates::matches_self_defending;
use super::tokens::FIXED;

pub const PASS: &str = "remove-js-confuser-self-defending";

const MARK: &str = "RegExp(\"\\n\")";

static SOURCE_TEXT: Reliance = Reliance {
    roots: &["RegExp", "Function", "Object", "String"],
    members: &[],
};

#[derive(Default)]
pub struct RemoveSelfDefending {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(RemoveSelfDefending::default())
}

fn tested_function(statement: &ExpressionStatement<'_>) -> Option<Span> {
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return None;
    };
    let Expression::FunctionExpression(outer) = call.callee.without_parentheses() else {
        return None;
    };
    let Some(Statement::VariableDeclaration(declaration)) = outer.body.as_ref()?.statements.first()
    else {
        return None;
    };
    match declaration.declarations.first()?.init.as_ref()? {
        Expression::FunctionExpression(tested) => Some(tested.span),
        _ => None,
    }
}

struct Find<'s> {
    source: &'s str,
    names: &'s Names,
    found: Vec<(Span, Span)>,
}

impl<'a> VisitJs<'a> for Find<'_> {
    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        let text = self
            .source
            .get(it.span.start as usize..it.span.end as usize)
            .unwrap_or("");
        if text.contains(MARK) && matches_self_defending(self.source, it.span, self.names) {
            if let Some(tested) = tested_function(it) {
                self.found.push((it.span, tested));
                return;
            }
        }
        walk_js::walk_expression_statement(self, it);
    }
}

fn closed(refs: &Refs<'_>, scoping: &Scoping, region: Span) -> bool {
    refs.all
        .iter()
        .filter(|(r, _)| inside(region, r.span))
        .all(|(r, name)| match r.symbol {
            None => FIXED.contains(name) && !is_foreign_global(name),
            Some(symbol) => inside(region, scoping.symbol_span(symbol)),
        })
}

fn removable(
    program: &Program<'_>,
    scoping: &Scoping,
    unsafe_lane: bool,
) -> Result<Vec<Span>, &'static str> {
    let names = names(program, scoping);
    let mut find = Find {
        source: program.source_text,
        names: &names,
        found: Vec::new(),
    };
    find.visit_program(program);
    if find.found.is_empty() {
        return Ok(Vec::new());
    }
    let refs = scan(program, scoping);
    if refs.has_with || may_call_eval(scoping) {
        return Err("the program has a with statement or may call eval");
    }
    let skipped: Vec<Span> = find.found.iter().map(|(statement, _)| *statement).collect();
    if !unsafe_lane && !unpatched_outside(program, scoping, &SOURCE_TEXT, &skipped) {
        return Err(
            "the program may replace RegExp or Function.prototype.toString, which the check uses",
        );
    }
    let mut out = Vec::new();
    for (statement, tested) in find.found {
        let source = program
            .source_text
            .get(tested.start as usize..tested.end as usize)
            .unwrap_or("\n");
        if source.contains('\n') {
            return Err("a checked function's source text holds a newline, so the check would hang the input");
        }
        if !closed(&refs, scoping, statement) {
            return Err("a check reads a name from outside");
        }
        out.push(statement);
    }
    Ok(out)
}

impl<'a> Pass<'a> for RemoveSelfDefending {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.settled || !program.source_text.contains(MARK) {
            return;
        }
        self.settled = true;
        match removable(program, cx.scoping(), cx.opts.unsafe_transforms) {
            Ok(spans) if spans.is_empty() => {}
            Ok(spans) => {
                let mut removals = StmtEdits::new();
                for span in &spans {
                    removals.remove(*span);
                }
                let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
                cx.changed_by(removed);
                cx.note(format!("{removed} js-confuser self-defending check(s) removed: each checked function's source text holds no newline, so the check never fires on the input"), None);
            }
            Err(reason) => cx.note(
                format!("js-confuser self-defending checks kept: {reason}"),
                None,
            ),
        }
    }
}
