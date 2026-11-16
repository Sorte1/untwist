use oxc_allocator::{Address, GetAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolFlags;

use super::{
    function_literal, member_key, same, transparent, Walker, GLOBAL_RECEIVER_READERS, NAMESPACE,
};

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

const REFLECT_APPLY_RECEIVER: usize = 1;

fn bare(pattern: &BindingPattern<'_>) -> bool {
    matches!(pattern, BindingPattern::BindingIdentifier(_))
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn global_flow_allowed(&self, mut at: usize) -> bool {
        let mut child = self.stack[at].address();
        while at > 0 {
            at -= 1;
            let parent = &self.stack[at];
            match parent {
                kind if transparent(kind) => {}
                AstKind::ConditionalExpression(_)
                | AstKind::LogicalExpression(_)
                | AstKind::SequenceExpression(_)
                | AstKind::AwaitExpression(_) => {}
                AstKind::VariableDeclarator(d) => return bare(&d.id),
                AstKind::AssignmentPattern(p) => return bare(&p.left),
                AstKind::FormalParameter(p) => return bare(&p.pattern),
                AstKind::AssignmentExpression(a) => {
                    return !matches!(
                        a.left,
                        AssignmentTarget::ObjectAssignmentTarget(_)
                            | AssignmentTarget::ArrayAssignmentTarget(_)
                    ) || !same(&a.right, child);
                }
                AstKind::AssignmentTargetWithDefault(t) => {
                    return !matches!(
                        t.binding,
                        AssignmentTarget::ObjectAssignmentTarget(_)
                            | AssignmentTarget::ArrayAssignmentTarget(_)
                    );
                }
                AstKind::CallExpression(call) => return !self.forwarded_receiver_slot(call, child),
                _ => return true,
            }
            child = parent.address();
        }
        true
    }

    fn forwarded_receiver_slot(&self, call: &CallExpression<'a>, child: Address) -> bool {
        let Some(index) = call.arguments.iter().position(|argument| {
            argument
                .as_expression()
                .is_some_and(|expr| same(expr, child))
        }) else {
            return false;
        };
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return false;
        };
        let Some(key) = member_key(member, self.resolve) else {
            return false;
        };
        let object = member.object();
        if self.kind_here(object) & NAMESPACE != 0 {
            return key == "apply" && index == REFLECT_APPLY_RECEIVER;
        }
        index == 0 && FORWARDERS.contains(&key.as_ref()) && !self.visible_function(object)
    }

    fn visible_function(&self, object: &Expression<'a>) -> bool {
        if function_literal(object) {
            return true;
        }
        match object.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id).is_some_and(|symbol| {
                let declared = self
                    .scoping
                    .symbol_flags(symbol)
                    .contains(SymbolFlags::Function)
                    || self
                        .candidates
                        .get(&symbol)
                        .is_some_and(|candidate| function_literal(candidate.init));
                declared && !self.scoping.symbol_is_mutated(symbol)
            }),
            other => other.as_member_expression().is_some_and(|reader| {
                member_key(reader, self.resolve)
                    .is_some_and(|name| GLOBAL_RECEIVER_READERS.contains(&name.as_ref()))
            }),
        }
    }
}
