use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_traverse::Ancestor;
use untwist_core::pass::Cx;

pub fn in_single_statement_slot(stmt: &Statement<'_>, cx: &Cx<'_, '_>) -> bool {
    match cx.tcx.parent() {
        Ancestor::IfStatementConsequent(_)
        | Ancestor::IfStatementAlternate(_)
        | Ancestor::DoWhileStatementBody(_)
        | Ancestor::WhileStatementBody(_)
        | Ancestor::ForStatementBody(_)
        | Ancestor::ForInStatementBody(_)
        | Ancestor::ForOfStatementBody(_)
        | Ancestor::WithStatementBody(_) => true,
        Ancestor::LabeledStatementBody(_) => !stmt.is_iteration_statement(),
        _ => false,
    }
}

pub fn labelled_loop<'s, 'a>(stmt: &'s Statement<'a>) -> Option<&'s Statement<'a>> {
    if !matches!(stmt, Statement::LabeledStatement(_)) {
        return None;
    }
    let mut current = stmt;
    while let Statement::LabeledStatement(labeled) = current {
        current = &labeled.body;
    }
    current.is_iteration_statement().then_some(current)
}

fn innermost_body<'s, 'a>(stmt: &'s mut Statement<'a>) -> &'s mut Statement<'a> {
    match stmt {
        Statement::LabeledStatement(labeled) => innermost_body(&mut labeled.body),
        other => other,
    }
}

pub fn hoist_in_front_of_labels<'a>(
    mut stmt: Statement<'a>,
    out: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
    expand: impl FnOnce(Statement<'a>, &mut AVec<'a, Statement<'a>>),
) {
    let body = innermost_body(&mut stmt);
    let placeholder = Statement::new_empty_statement(body.span(), cx);
    let taken = std::mem::replace(body, placeholder);
    let mut expanded = AVec::new_in(cx);
    expand(taken, &mut expanded);
    if let Some(last) = expanded.pop() {
        *body = last;
    }
    out.extend(expanded);
    out.push(stmt);
}
