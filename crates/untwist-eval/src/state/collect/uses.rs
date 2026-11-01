use oxc_ast::ast::{Expression, VariableDeclarator};
use oxc_span::Span;

use super::judge::Walker;
use super::scope::Freshness;
use super::{Allowance, BareUse, TriggerKind};

#[derive(Clone, Copy)]
pub(super) enum UseKind {
    Plain,
    Argument(usize),
    Alias(usize),
}

pub(super) struct PendingUse<'a> {
    name: &'a str,
    kind: UseKind,
    in_region: bool,
    at: u32,
}

impl<'a> Walker<'a, '_> {
    pub(super) fn note_bare_machinery(&mut self, name: &'a str, span: Span) {
        if self.watched.contains(&name) {
            let kind = match (self.alias_of.take(), self.local_callee.take()) {
                (Some(alias), _) => UseKind::Alias(alias),
                (None, Some(callee)) => UseKind::Argument(callee),
                (None, None) => UseKind::Plain,
            };
            let use_ = PendingUse {
                name,
                kind,
                in_region: self.in_region(),
                at: self.access_at(span),
            };
            self.pending_bare.push(use_);
        }
        if self.machinery.contains(&name) {
            self.push_trigger(span, TriggerKind::Monitored);
        }
    }

    pub(super) fn alias_binding(&self, it: &VariableDeclarator<'a>) -> Option<usize> {
        let init = it.init.as_ref()?;
        let Expression::Identifier(source) = init.without_parentheses() else {
            return None;
        };
        if !self.watched.contains(&source.name.as_str()) {
            return None;
        }
        let id = it.id.get_binding_identifier()?;
        self.resolve(id.name.as_str())
    }

    fn is_local_function(&self, binding: usize) -> bool {
        let entry = &self.bindings[binding];
        entry.fresh == Freshness::Function && entry.declarations == 1 && entry.writes == 0
    }

    fn is_call_only_alias(&self, binding: usize) -> bool {
        let entry = &self.bindings[binding];
        entry.declarations == 1 && entry.writes == 0 && !entry.non_call_use
    }

    fn allowance(&self, use_: &PendingUse<'_>) -> Allowance {
        match use_.kind {
            UseKind::Alias(binding) if self.is_call_only_alias(binding) => Allowance::Always,
            UseKind::Argument(binding) if self.is_local_function(binding) => {
                if use_.in_region {
                    Allowance::Always
                } else {
                    Allowance::AfterGates(use_.at)
                }
            }
            _ => Allowance::Never,
        }
    }

    pub(super) fn judge_uses(&mut self) {
        for use_ in std::mem::take(&mut self.pending_bare) {
            let allowance = self.allowance(&use_);
            self.out.bare_uses.push(BareUse {
                name: use_.name,
                allowance,
            });
        }
        for (root, key, value) in std::mem::take(&mut self.pending_writes) {
            let function = value
                .is_some_and(|binding| binding == usize::MAX || self.is_local_function(binding));
            self.out.function_writes.push((root, key, function));
        }
    }
}
