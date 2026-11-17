use oxc_allocator::{Address, GetAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolFlags;
use std::borrow::Cow;

use super::{
    function_literal, member_address, member_key, same, transparent, unsafe_read, Walker,
    GLOBAL_RECEIVER_READERS, KIND_DEPTH, NAMESPACE, VALUE,
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
                AstKind::VariableDeclarator(_) => return true,
                AstKind::AssignmentPattern(p) => return bare(&p.left),
                AstKind::FormalParameter(p) => return bare(&p.pattern),
                AstKind::AssignmentExpression(_) => return true,
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

    pub(super) fn member_use(&self, at: usize, kind: u8) -> bool {
        if kind & VALUE != 0 {
            return false;
        }
        if kind & NAMESPACE == 0 {
            return true;
        }
        let member = &self.stack[at];
        let address = member.address();
        let key = match member {
            AstKind::StaticMemberExpression(m) => Some(Cow::Borrowed(m.property.name.as_str())),
            AstKind::ComputedMemberExpression(m) => {
                self.key_text(&m.expression, KIND_DEPTH).map(Cow::Owned)
            }
            _ => None,
        };
        if self.called_or_written(at, address) {
            return true;
        }
        if key.as_deref().is_some_and(|key| !unsafe_read(key)) {
            return true;
        }
        self.allowed(at, VALUE)
    }

    fn called_or_written(&self, at: usize, member: Address) -> bool {
        let mut child = member;
        let mut up = at;
        while up > 0 {
            up -= 1;
            match &self.stack[up] {
                kind if transparent(kind) => child = kind.address(),
                AstKind::CallExpression(c) => return c.callee.address() == child,
                AstKind::NewExpression(n) => return n.callee.address() == child,
                AstKind::AssignmentExpression(a) => {
                    return a
                        .left
                        .as_member_expression()
                        .is_some_and(|m| member_address(m) == member)
                }
                AstKind::UpdateExpression(_)
                | AstKind::ArrayAssignmentTarget(_)
                | AstKind::AssignmentTargetWithDefault(_)
                | AstKind::AssignmentTargetPropertyProperty(_)
                | AstKind::AssignmentTargetRest(_) => return true,
                AstKind::ForInStatement(f) => return !same(&f.right, child),
                AstKind::ForOfStatement(f) => return !same(&f.right, child),
                _ => return false,
            }
        }
        false
    }
}
