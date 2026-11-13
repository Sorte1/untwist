use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::member_parts;

use super::{cff_props, contains, CffValue, MAX_NEST};

pub(super) fn is_nest_site(expr: &Expression<'_>, extended: bool) -> bool {
    let Some((callee, _)) = call_parts(expr) else {
        return false;
    };
    let Some((object, name)) = member_parts(callee) else {
        return false;
    };
    cff_props(object, extended).is_some_and(|props| {
        props
            .iter()
            .find(|(key, _)| *key == name)
            .is_some_and(|(_, value)| matches!(value, CffValue::Function(_)))
    })
}

pub(super) fn doomed_sites(root: &Expression<'_>, extended: bool) -> FxHashSet<Span> {
    let mut chains = Chains {
        extended,
        open: Vec::new(),
        sites: Vec::new(),
        tangled: false,
    };
    chains.visit_expression(root);
    if chains.tangled {
        return FxHashSet::default();
    }
    chains
        .sites
        .into_iter()
        .filter(|site| site.depth + site.below > MAX_NEST)
        .map(|site| site.span)
        .collect()
}

struct Site {
    span: Span,
    depth: usize,
    below: usize,
}

struct Chains {
    extended: bool,
    open: Vec<usize>,
    sites: Vec<Site>,
    tangled: bool,
}

impl<'a> Visit<'a> for Chains {
    fn visit_expression(&mut self, expr: &Expression<'a>) {
        if self.tangled || !is_nest_site(expr, self.extended) {
            walk::walk_expression(self, expr);
            return;
        }
        let span = expr.span();
        let parent = self.open.last().map(|&index| self.sites[index].span);
        if span.start >= span.end
            || parent.is_some_and(|outer| outer == span || !contains(outer, span))
        {
            self.tangled = true;
            return;
        }
        let index = self.sites.len();
        self.sites.push(Site {
            span,
            depth: self.open.len() + 1,
            below: 0,
        });
        self.open.push(index);
        walk::walk_expression(self, expr);
        self.open.pop();
        let reach = self.sites[index].below + 1;
        if let Some(&outer) = self.open.last() {
            let outer = &mut self.sites[outer];
            outer.below = outer.below.max(reach);
        }
    }
}
