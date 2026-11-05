use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use untwist_core::ast_util::scope::{binding_symbol, references_symbol};
use untwist_core::pass::{Cx, Interest, Locality, Pass};

#[derive(Default)]
pub struct SplitForLoopVars;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(SplitForLoopVars)
}

fn is_candidate(stmt: &Statement<'_>) -> bool {
    let Statement::ForStatement(node) = stmt else {
        return false;
    };
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &node.init else {
        return false;
    };
    matches!(declaration.kind, VariableDeclarationKind::Var)
        && !declaration.declarations.is_empty()
        && declaration
            .declarations
            .iter()
            .all(|declarator| matches!(declarator.id, BindingPattern::BindingIdentifier(_)))
}

fn hoistable_prefix_len(node: &ForStatement<'_>, scoping: &Scoping) -> usize {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &node.init else {
        return 0;
    };
    let mut count = 0;
    for declarator in &declaration.declarations {
        let Some(symbol) = binding_symbol(&declarator.id) else {
            break;
        };
        let used_in_test = node
            .test
            .as_ref()
            .is_some_and(|test| references_symbol(test, scoping, symbol));
        let used_in_update = node
            .update
            .as_ref()
            .is_some_and(|update| references_symbol(update, scoping, symbol));
        if used_in_test || used_in_update {
            break;
        }
        count += 1;
    }
    count
}

fn hoist_prefix<'a>(
    mut node: oxc_allocator::Box<'a, ForStatement<'a>>,
    count: usize,
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> Statement<'a> {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &mut node.init else {
        return Statement::ForStatement(node);
    };
    let kind = declaration.kind;
    let declare = declaration.declare;
    let taken = std::mem::replace(&mut declaration.declarations, AVec::new_in(&*cx));
    let mut remaining = taken.into_iter();
    for _ in 0..count {
        let Some(declarator) = remaining.next() else {
            break;
        };
        let span = declarator.span;
        let single = AVec::from_iter_in([declarator], &*cx);
        stmts.push(Statement::new_variable_declaration(
            span, kind, single, declare, &*cx,
        ));
        cx.changed();
    }
    for declarator in remaining {
        declaration.declarations.push(declarator);
    }
    if declaration.declarations.is_empty() {
        node.init = None;
    }
    Statement::ForStatement(node)
}

impl<'a> Pass<'a> for SplitForLoopVars {
    fn name(&self) -> &'static str {
        "split-for-loop-vars"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENTS
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !stmts.iter().any(is_candidate) {
            return;
        }
        let capacity = stmts.len() + 4;
        let old = std::mem::replace(stmts, AVec::with_capacity_in(capacity, &*cx));
        for stmt in old {
            let count = match &stmt {
                Statement::ForStatement(node) if is_candidate(&stmt) => {
                    hoistable_prefix_len(node, cx.scoping())
                }
                _ => 0,
            };
            if count == 0 {
                stmts.push(stmt);
                continue;
            }
            let Statement::ForStatement(node) = stmt else {
                unreachable!()
            };
            let result = hoist_prefix(node, count, stmts, cx);
            stmts.push(result);
        }
    }
}
