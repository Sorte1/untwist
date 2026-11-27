use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::ast::Ctx;
use super::shape::{object_member, Prop, Use};

#[derive(Clone)]
pub(super) struct Site {
    pub(super) name: String,
    pub(super) typeof_read: bool,
    pub(super) target: SymbolId,
}

#[derive(Default)]
pub(super) struct Sites {
    pub(super) expressions: FxHashMap<Span, Site>,
    pub(super) targets: FxHashMap<Span, Site>,
    pub(super) bad: bool,
}

struct Scan<'s, 'p> {
    cx: Ctx<'s>,
    object: SymbolId,
    props: &'p FxHashMap<String, Prop>,
    sites: Sites,
}

fn site(prop: &Prop) -> Site {
    Site {
        name: prop.name.clone(),
        typeof_read: prop.kind == Use::Typeof,
        target: prop.target,
    }
}

impl Scan<'_, '_> {
    fn prop(&self, expr: &Expression<'_>) -> Option<Prop> {
        object_member(&self.cx, expr, self.object, self.props).cloned()
    }
}

impl<'a> VisitJs<'a> for Scan<'_, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        match self.prop(it) {
            Some(prop) if prop.kind != Use::Call => {
                self.sites
                    .expressions
                    .insert(it.without_parentheses().span(), site(&prop));
            }
            Some(_) => self.sites.bad = true,
            None => walk_js::walk_expression(self, it),
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        match self.prop(&it.callee) {
            Some(prop) if prop.kind == Use::Call && !it.optional => {
                self.sites
                    .expressions
                    .insert(it.callee.without_parentheses().span(), site(&prop));
                for argument in &it.arguments {
                    self.visit_argument(argument);
                }
            }
            Some(_) => self.sites.bad = true,
            None => walk_js::walk_call_expression(self, it),
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        let SimpleAssignmentTarget::ComputedMemberExpression(member) = it else {
            walk_js::walk_simple_assignment_target(self, it);
            return;
        };
        let Some(key) = super::super::ast::string(&member.expression) else {
            walk_js::walk_simple_assignment_target(self, it);
            return;
        };
        if !self.cx.is(&member.object, self.object) {
            walk_js::walk_simple_assignment_target(self, it);
            return;
        }
        match self.props.get(&key) {
            Some(prop) if prop.kind == Use::Standard && prop.settable => {
                self.sites.targets.insert(member.span, site(prop));
            }
            _ => self.sites.bad = true,
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete && self.prop(&it.argument).is_some() {
            self.sites.bad = true;
            return;
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if self.prop(&it.tag).is_some() {
            self.sites.bad = true;
            return;
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.cx.scoping.get_reference(id).symbol_id());
        self.sites.bad |= symbol == Some(self.object);
    }
}

pub(super) fn scan(
    cx: Ctx<'_>,
    function: &Function<'_>,
    object: SymbolId,
    props: &FxHashMap<String, Prop>,
) -> Sites {
    let mut scan = Scan {
        cx,
        object,
        props,
        sites: Sites::default(),
    };
    walk_js::walk_function(&mut scan, function, oxc_syntax::scope::ScopeFlags::Function);
    scan.sites
}
