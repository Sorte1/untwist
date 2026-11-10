use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use untwist_core::pass::{Cx, Interest, Locality, Pass, StatementKind, StatementVariants};

use crate::statement_slot::{hoist_in_front_of_labels, in_single_statement_slot, labelled_loop};

#[derive(Default)]
pub struct SplitVariableDeclarations;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(SplitVariableDeclarations)
}

fn is_multi_declaration(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.declarations.len() > 1
}

fn is_splittable_for(node: &ForStatement<'_>) -> bool {
    if node.test.is_some() || node.update.is_some() {
        return false;
    }
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &node.init else {
        return false;
    };
    matches!(declaration.kind, VariableDeclarationKind::Var) && is_multi_declaration(declaration)
}

fn is_splittable_export(node: &ExportDeclaration<'_>) -> bool {
    matches!(&node.declaration, Declaration::VariableDeclaration(declaration)
        if is_multi_declaration(declaration))
}

fn needs_split(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::VariableDeclaration(declaration) => is_multi_declaration(declaration),
        Statement::ForStatement(node) => is_splittable_for(node),
        Statement::ExportDeclaration(node) => is_splittable_export(node),
        _ => false,
    }
}

fn needs_list_split(stmt: &Statement<'_>) -> bool {
    needs_split(stmt) || labelled_loop(stmt).is_some_and(needs_split)
}

fn split_span(start: &mut Option<u32>, declarator: Span) -> Span {
    match start.take() {
        Some(start) if start <= declarator.start => Span::new(start, declarator.end),
        _ => declarator,
    }
}

fn push_single_declarations<'a>(
    out: &mut AVec<'a, Statement<'a>>,
    declaration: VariableDeclaration<'a>,
    cx: &Cx<'a, '_>,
) {
    let kind = declaration.kind;
    let declare = declaration.declare;
    let mut start = Some(declaration.span.start);
    for declarator in declaration.declarations {
        let span = split_span(&mut start, declarator.span);
        let single = AVec::from_iter_in([declarator], cx);
        out.push(Statement::new_variable_declaration(
            span, kind, single, declare, cx,
        ));
    }
}

fn split_statement<'a>(stmt: Statement<'a>, out: &mut AVec<'a, Statement<'a>>, cx: &Cx<'a, '_>) {
    if labelled_loop(&stmt).is_some() {
        hoist_in_front_of_labels(stmt, out, cx, |body, out| split_statement(body, out, cx));
        return;
    }
    match stmt {
        Statement::VariableDeclaration(declaration) => {
            push_single_declarations(out, declaration.unbox(), cx);
        }
        Statement::ExportDeclaration(export) => {
            if !matches!(export.declaration, Declaration::VariableDeclaration(_)) {
                out.push(Statement::ExportDeclaration(export));
                return;
            }
            let export = export.unbox();
            let Declaration::VariableDeclaration(declaration) = export.declaration else {
                unreachable!()
            };
            let declaration = declaration.unbox();
            let kind = declaration.kind;
            let declare = declaration.declare;
            let mut start = Some(export.span.start);
            for declarator in declaration.declarations {
                let span = declarator.span;
                let outer = split_span(&mut start, span);
                let single = AVec::from_iter_in([declarator], cx);
                let inner = Declaration::new_variable_declaration(span, kind, single, declare, cx);
                out.push(Statement::new_export_declaration(outer, inner, cx));
            }
        }
        Statement::ForStatement(mut node) => {
            if let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.take() {
                push_single_declarations(out, declaration.unbox(), cx);
            }
            out.push(Statement::ForStatement(node));
        }
        other => out.push(other),
    }
}

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[
    StatementKind::VariableDeclaration,
    StatementKind::ForStatement,
    StatementKind::ExportDeclaration,
]);

impl<'a> Pass<'a> for SplitVariableDeclarations {
    fn name(&self) -> &'static str {
        "split-variable-declarations"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_STATEMENTS | Interest::EXIT_STATEMENT).on_statements(ACTED_ON_STATEMENTS)
    }

    fn enter_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !stmts.iter().any(needs_list_split) {
            return;
        }
        let capacity = stmts.len() + 4;
        let old = std::mem::replace(stmts, AVec::with_capacity_in(capacity, &*cx));
        for stmt in old {
            if !needs_list_split(&stmt) {
                stmts.push(stmt);
                continue;
            }
            split_statement(stmt, stmts, &*cx);
            cx.changed();
        }
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if !in_single_statement_slot(stmt, cx) || !needs_split(stmt) {
            return;
        }
        if matches!(stmt, Statement::VariableDeclaration(node) if node.kind.is_lexical()) {
            cx.binding_moved();
        }
        let span = stmt.span();
        let placeholder = Statement::new_empty_statement(span, &*cx);
        let taken = std::mem::replace(stmt, placeholder);
        let mut body = AVec::new_in(&*cx);
        split_statement(taken, &mut body, &*cx);
        *stmt = cx.block(span, body);
        cx.changed();
    }
}
