use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::pass::{Cx, Interest, NoteKind, Pass, RepeatedSites, Safety};

use crate::build;
use crate::common::{self, Convert};
use crate::notes;

pub const PASS: &str = "jsx";
const PRAGMA: &str = "createElement";

#[derive(Default)]
pub struct Jsx {
    bindings: crate::pragma::ReactBindings,
    attribute_shape: RepeatedSites,
    element_name_shape: RepeatedSites,
    component_looking_string: RepeatedSites,
    lowercase_unbound: RepeatedSites,
    component_name_exhausted: RepeatedSites,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Jsx::default())
}

fn props_are_convertible(props: Option<&Expression<'_>>) -> bool {
    matches!(
        props,
        Some(Expression::ObjectExpression(_) | Expression::NullLiteral(_))
    )
}

pub(crate) enum Plan {
    Fragment,
    Element(build::NamePlan),
}

impl<'a> Convert<'a> for Jsx {
    type Plan = Plan;

    fn plan(&mut self, expr: &Expression<'a>, cx: &mut Cx<'a, '_>) -> Option<Plan> {
        let (callee, arguments) = call_parts(expr)?;
        if !self.bindings.names_member(callee, PRAGMA, cx) || arguments.len() < 2 {
            return None;
        }
        let type_expr = arguments.first()?.as_expression()?;
        let props = arguments.get(1).and_then(Argument::as_expression);
        if !props_are_convertible(props) {
            return None;
        }
        if let Some(shape) = build::inconvertible_prop(props) {
            self.attribute_shape.record(expr.span(), shape);
            return None;
        }
        if self.bindings.names_member(type_expr, build::FRAGMENT, cx)
            && matches!(props, Some(Expression::NullLiteral(_)))
        {
            return Some(Plan::Fragment);
        }
        let renamed = common::plan_rename(
            false,
            type_expr,
            &mut self.lowercase_unbound,
            &mut self.component_name_exhausted,
            cx,
        )?;
        let name = match renamed {
            Some(renamed) => build::NamePlan::Tag(renamed),
            None => build::name_plan(type_expr)?,
        };
        if let Some(unspellable) = build::unspellable_tag(&name) {
            self.element_name_shape.record(expr.span(), unspellable);
            return None;
        }
        let conservative = !cx.opts.extended;
        if let Some(component) = build::component_looking_string(type_expr, conservative) {
            self.component_looking_string.record(expr.span(), component);
            return None;
        }
        Some(Plan::Element(name))
    }

    fn rewrite(&mut self, expr: &mut Expression<'a>, plan: Plan, cx: &mut Cx<'a, '_>) -> bool {
        let Some((span, mut arguments)) = common::take_call(expr, &*cx) else {
            return false;
        };
        let rest: Vec<Argument<'a>> = arguments.drain(2..).collect();
        let conservative = !cx.opts.extended;
        let mut children = AVec::with_capacity_in(rest.len(), &*cx);
        for argument in rest {
            children.push(build::child(argument, conservative, &*cx));
        }
        let props = arguments.pop();
        *expr = match plan {
            Plan::Fragment => build::fragment(children, span, &*cx),
            Plan::Element(plan) => {
                let attributes = attributes_of(props, &*cx);
                let name = build::element_name(&plan, &*cx);
                build::element(name, attributes, children, span, &*cx)
            }
        };
        true
    }
}

impl Jsx {
    fn report(&mut self, cx: &mut Cx<'_, '_>) {
        common::emit_notes(
            &mut [
                (
                    &mut self.attribute_shape,
                    notes::ATTRIBUTE_SHAPE,
                    NoteKind::Refusal,
                ),
                (
                    &mut self.element_name_shape,
                    notes::ELEMENT_NAME_SHAPE,
                    NoteKind::Refusal,
                ),
                (
                    &mut self.component_looking_string,
                    notes::COMPONENT_LOOKING_STRING,
                    NoteKind::Refusal,
                ),
                (
                    &mut self.lowercase_unbound,
                    notes::LOWERCASE_UNBOUND,
                    NoteKind::Refusal,
                ),
                (
                    &mut self.component_name_exhausted,
                    notes::COMPONENT_NAME_EXHAUSTED,
                    NoteKind::Refusal,
                ),
            ],
            cx,
        );
    }
}

fn attributes_of<'a>(
    props: Option<Argument<'a>>,
    cx: &Cx<'a, '_>,
) -> AVec<'a, JSXAttributeItem<'a>> {
    match props {
        Some(Argument::ObjectExpression(object)) => build::attributes(object, false, cx),
        _ => AVec::new_in(cx),
    }
}

impl<'a> Pass<'a> for Jsx {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION | Interest::EXIT_PROGRAM
    }

    fn safety(&self) -> Safety {
        Safety::Unsafe
    }

    fn reset(&mut self) {
        self.bindings = crate::pragma::ReactBindings::default();
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.bindings = match cx.opts.extended {
            true => crate::pragma::react_bindings(program),
            false => crate::pragma::ReactBindings::default(),
        };
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        common::convert(self, expr, cx);
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.report(cx);
        common::apply_renames(program, cx);
    }
}
