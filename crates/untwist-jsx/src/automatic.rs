use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::{identifier_name, member_parts, prop_name};
use untwist_core::pass::{Cx, Interest, NoteKind, Pass, RepeatedSites, Safety};

use crate::build;
use crate::common::{self, Convert};
use crate::notes;
use crate::order;
use crate::pragma::{react_bindings, ReactBindings};
use crate::uid::{component_name, COMPONENT};

pub const PASS: &str = "jsx-new";
pub const PRAGMAS: [&str; 6] = ["jsx", "jsxs", "_jsx", "_jsxs", "jsxDEV", "jsxsDEV"];
const PLURAL: &str = "jsxs";
const KEY_ARITY: usize = 3;

fn pragma_of(callee: &Expression<'_>) -> Option<&'static str> {
    let name = direct_pragma(callee).or_else(|| indirect_pragma(callee))?;
    PRAGMAS.iter().find(|known| **known == name).copied()
}

fn direct_pragma<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    if let Some(name) = identifier_name(callee) {
        return Some(name);
    }
    let (object, name) = member_parts(callee)?;
    identifier_name(object)?;
    match name {
        std::borrow::Cow::Borrowed(name) => Some(name),
        std::borrow::Cow::Owned(_) => None,
    }
}

fn indirect_pragma<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    let Expression::SequenceExpression(sequence) = callee else {
        return None;
    };
    let [Expression::NumericLiteral(zero), member] = sequence.expressions.as_slice() else {
        return None;
    };
    if zero.value != 0.0 {
        return None;
    }
    direct_pragma(member)
}

pub(crate) fn is_pragma_callee(callee: &Expression<'_>) -> bool {
    pragma_of(callee).is_some()
}

fn plural(pragma: &str) -> bool {
    pragma.contains(PLURAL)
}

pub(crate) struct Plan {
    pragma: &'static str,
    name: build::NamePlan,
    hoisted: Option<(String, Span)>,
    fragment: bool,
}

#[derive(Default)]
pub struct JsxNew<'a> {
    bindings: ReactBindings,
    hoists: Vec<(Span, Statement<'a>)>,
    statement_spans: Vec<Span>,
    list_marks: Vec<(usize, ScopeId)>,
    first_calls: FxHashSet<Span>,
    hoist_order: RepeatedSites,
    attribute_shape: RepeatedSites,
    element_name_shape: RepeatedSites,
    component_looking_string: RepeatedSites,
    lowercase_unbound: RepeatedSites,
    type_hoisted: RepeatedSites,
    hoist_anchor_unresolved: RepeatedSites,
    key_shape: RepeatedSites,
    children_shape: RepeatedSites,
    component_name_exhausted: RepeatedSites,
}

struct FreeIdentifiers<'s> {
    scoping: &'s Scoping,
    declared: FxHashSet<SymbolId>,
    referenced: Vec<(SymbolId, String)>,
}

impl<'a> VisitJs<'a> for FreeIdentifiers<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.declared.insert(symbol);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol {
            self.referenced.push((symbol, it.name.to_string()));
        }
    }
}

fn unresolved_at(type_expr: &Expression<'_>, anchor: ScopeId, scoping: &Scoping) -> Option<String> {
    let mut scan = FreeIdentifiers {
        scoping,
        declared: FxHashSet::default(),
        referenced: Vec::new(),
    };
    scan.visit_expression(type_expr);
    let visible: FxHashSet<ScopeId> = scoping.scope_ancestors(anchor).collect();
    let declared = scan.declared;
    scan.referenced
        .into_iter()
        .find(|(symbol, _)| {
            !declared.contains(symbol) && !visible.contains(&scoping.symbol_scope_id(*symbol))
        })
        .map(|(_, name)| name)
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(JsxNew::default())
}

impl<'a> Convert<'a> for JsxNew<'a> {
    type Plan = Plan;

    fn plan(&mut self, expr: &Expression<'a>, cx: &mut Cx<'a, '_>) -> Option<Plan> {
        let (callee, arguments) = call_parts(expr)?;
        let pragma = pragma_of(callee)?;
        if arguments.len() < 2 {
            return None;
        }
        if arguments.len() > KEY_ARITY {
            let arity = format!("{} arguments", arguments.len());
            self.key_shape.record(expr.span(), &arity);
            return None;
        }
        let type_expr = arguments.first()?.as_expression()?;
        let props = arguments.get(1)?.as_expression()?;
        if !matches!(props, Expression::ObjectExpression(_)) {
            return None;
        }
        if arguments.len() == KEY_ARITY && arguments.get(2)?.as_expression().is_none() {
            return None;
        }
        if let Some(shape) = build::inconvertible_prop(Some(props)) {
            self.attribute_shape.record(expr.span(), shape);
            return None;
        }
        let conservative = !cx.opts.extended;
        if !conservative && arguments.len() == KEY_ARITY && has_prop(props, KEY) {
            self.key_shape
                .record(expr.span(), "key in the props and as an argument");
            return None;
        }
        if let Some(shape) = children_shape(props, pragma, conservative) {
            self.children_shape.record(expr.span(), shape);
            return None;
        }
        let fragment = self.bindings.names_member(type_expr, build::FRAGMENT, cx);
        let renamed = common::plan_rename(
            conservative,
            type_expr,
            &mut self.lowercase_unbound,
            &mut self.component_name_exhausted,
            cx,
        )?;
        let (name, hoisted) = self.plan_name(type_expr, renamed, expr.span(), cx)?;
        Some(Plan {
            pragma,
            name,
            hoisted,
            fragment,
        })
    }

    fn rewrite(&mut self, expr: &mut Expression<'a>, plan: Plan, cx: &mut Cx<'a, '_>) -> bool {
        let Some((span, mut arguments)) = common::take_call(expr, &*cx) else {
            return false;
        };
        let key = (arguments.len() == KEY_ARITY).then(|| arguments.remove(2));
        let (mut attributes, children_value) = props_of(arguments.pop(), &*cx);
        let conservative = !cx.opts.extended;
        let children = children_of(children_value, plan.pragma, conservative, &*cx);
        if let Some(attribute) = key.and_then(|argument| build::key_attribute(argument, &*cx)) {
            attributes.push(attribute);
        }
        if let Some((uid, anchor)) = &plan.hoisted {
            self.hoist(uid, *anchor, arguments.pop(), &*cx);
        }
        *expr = match plan.fragment && attributes.is_empty() {
            true => build::fragment(children, span, &*cx),
            false => {
                let name = build::element_name(&plan.name, &*cx);
                build::element(name, attributes, children, span, &*cx)
            }
        };
        true
    }
}

impl<'a> JsxNew<'a> {
    fn hoist_anchor(&self) -> Option<(Span, ScopeId)> {
        let (member, scope) = *self.list_marks.last()?;
        let anchor = self.statement_spans.get(member).copied()?;
        Some((anchor, scope))
    }

    fn plan_name(
        &mut self,
        type_expr: &Expression<'a>,
        renamed: Option<String>,
        span: Span,
        cx: &mut Cx<'a, '_>,
    ) -> Option<(build::NamePlan, Option<(String, Span)>)> {
        if let Some(name) = renamed {
            return Some((build::NamePlan::Tag(name), None));
        }
        if let Some(plan) = build::name_plan(type_expr) {
            if let Some(unspellable) = build::unspellable_tag(&plan) {
                self.element_name_shape.record(span, unspellable);
                return None;
            }
            let conservative = !cx.opts.extended;
            if let Some(component) = build::component_looking_string(type_expr, conservative) {
                self.component_looking_string.record(span, component);
                return None;
            }
            return Some((plan, None));
        }
        let (anchor, scope) = self.hoist_anchor()?;
        if cx.opts.extended {
            if let Some(name) = unresolved_at(type_expr, scope, cx.scoping()) {
                self.hoist_anchor_unresolved.record(type_expr.span(), &name);
                return None;
            }
            if !self.first_calls.contains(&span) {
                self.hoist_order.record(type_expr.span(), "type");
                return None;
            }
        }
        let Some(uid) = component_name(false, cx) else {
            self.component_name_exhausted
                .record(type_expr.span(), COMPONENT);
            return None;
        };
        self.type_hoisted.record(type_expr.span(), &uid);
        Some((build::NamePlan::Tag(uid.clone()), Some((uid, anchor))))
    }

    fn hoist(
        &mut self,
        uid: &str,
        anchor: Span,
        type_argument: Option<Argument<'a>>,
        b: &impl GetAstBuilder<'a>,
    ) {
        let init = match type_argument {
            None | Some(Argument::SpreadElement(_)) => return,
            Some(other) => other.into_expression(),
        };
        let name = Ident::from_str_in(uid, b.builder());
        let pattern = BindingPattern::new_binding_identifier(SPAN, name, b);
        let declarator = VariableDeclarator::new(SPAN, pattern, None, Some(init), false, b);
        let declarators = AVec::from_iter_in([declarator], b.builder());
        let statement = Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            declarators,
            false,
            b,
        );
        self.hoists.push((anchor, statement));
    }

    fn drain_hoists(&mut self, list: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.hoists.is_empty() {
            return;
        }
        let mut out = AVec::with_capacity_in(list.len(), &*cx);
        for statement in std::mem::replace(list, AVec::new_in(&*cx)) {
            let span = statement.span();
            while let Some(index) = self.hoists.iter().position(|(at, _)| *at == span) {
                out.push(self.hoists.remove(index).1);
            }
            out.push(statement);
        }
        *list = out;
    }

    fn land_hoists(&mut self, list: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        self.drain_hoists(list, cx);
        if self.hoists.is_empty() {
            return;
        }
        let unplaced = self.hoists.len();
        self.hoists.clear();
        cx.note(notes::hoists_not_placed(unplaced), None);
    }

    fn report(&mut self, cx: &mut Cx<'a, '_>) {
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
                    &mut self.type_hoisted,
                    notes::TYPE_HOISTED,
                    NoteKind::Progress,
                ),
                (
                    &mut self.hoist_anchor_unresolved,
                    notes::HOIST_ANCHOR_UNRESOLVED,
                    NoteKind::Refusal,
                ),
                (&mut self.key_shape, notes::KEY_SHAPE, NoteKind::Refusal),
                (&mut self.hoist_order, notes::HOIST_ORDER, NoteKind::Refusal),
                (
                    &mut self.children_shape,
                    notes::CHILDREN_SHAPE,
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

const KEY: &str = "key";

fn is_prop(property: &ObjectPropertyKind<'_>, wanted: &str) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    prop_name(&property.key).is_some_and(|name| name == wanted)
}

fn is_children_prop(property: &ObjectPropertyKind<'_>) -> bool {
    is_prop(property, build::CHILDREN)
}

fn has_prop(props: &Expression<'_>, wanted: &str) -> bool {
    matches!(props, Expression::ObjectExpression(object)
        if object.properties.iter().any(|property| is_prop(property, wanted)))
}

fn has_spread(value: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = value else {
        return false;
    };
    array
        .elements
        .iter()
        .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_)))
}

fn has_elision(value: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = value else {
        return false;
    };
    array
        .elements
        .iter()
        .any(|element| matches!(element, ArrayExpressionElement::Elision(_)))
}

fn children_shape(
    props: &Expression<'_>,
    pragma: &str,
    conservative: bool,
) -> Option<&'static str> {
    let Expression::ObjectExpression(object) = props else {
        return None;
    };
    match object
        .properties
        .iter()
        .filter(|p| is_children_prop(p))
        .count()
    {
        0 => return None,
        1 => {}
        _ => return Some("children appears more than once"),
    }
    let last = object.properties.last()?;
    if !is_children_prop(last) {
        return Some("children is not the last prop");
    }
    let ObjectPropertyKind::ObjectProperty(last) = last else {
        return None;
    };
    if !conservative && plural(pragma) && has_spread(&last.value) {
        return Some("children array has a spread");
    }
    (plural(pragma) && has_elision(&last.value)).then_some("children array has holes")
}

fn take_prop<'a>(
    object: &mut AVec<'a, ObjectPropertyKind<'a>>,
    wanted: &str,
) -> Option<Expression<'a>> {
    let index = object.iter().position(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        prop_name(&property.key).is_some_and(|name| name == wanted)
    })?;
    let ObjectPropertyKind::ObjectProperty(property) = object.remove(index) else {
        return None;
    };
    Some(property.unbox().value)
}

fn props_of<'a>(
    props: Option<Argument<'a>>,
    b: &impl GetAstBuilder<'a>,
) -> (AVec<'a, JSXAttributeItem<'a>>, Option<Expression<'a>>) {
    let Some(Argument::ObjectExpression(mut object)) = props else {
        return (AVec::new_in(b.builder()), None);
    };
    let children = take_prop(&mut object.properties, build::CHILDREN);
    (build::attributes(object, true, b), children)
}

fn children_of<'a>(
    value: Option<Expression<'a>>,
    pragma: &str,
    conservative: bool,
    b: &impl GetAstBuilder<'a>,
) -> AVec<'a, JSXChild<'a>> {
    let mut children = AVec::new_in(b.builder());
    let Some(value) = value else {
        return children;
    };
    match value {
        Expression::ArrayExpression(array) if plural(pragma) => {
            for element in array.unbox().elements {
                match element {
                    ArrayExpressionElement::Elision(_) => {}
                    ArrayExpressionElement::SpreadElement(spread) => {
                        children.push(JSXChild::new_spread(SPAN, spread.unbox().argument, b));
                    }
                    other => {
                        children.push(build::child_of(other.into_expression(), conservative, b))
                    }
                }
            }
        }
        other => children.push(build::child_of(other, conservative, b)),
    }
    children
}

impl<'a> Pass<'a> for JsxNew<'a> {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
            | Interest::ENTER_STATEMENTS
            | Interest::ENTER_STATEMENT
            | Interest::EXIT_STATEMENT
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_STATEMENTS
            | Interest::EXIT_PROGRAM
    }

    fn safety(&self) -> Safety {
        Safety::Unsafe
    }

    fn reset(&mut self) {
        self.bindings = ReactBindings::default();
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.bindings = match cx.opts.extended {
            true => react_bindings(program),
            false => ReactBindings::default(),
        };
        self.statement_spans.clear();
        self.list_marks.clear();
        self.first_calls.clear();
    }

    fn enter_statements(&mut self, _node: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        self.list_marks
            .push((self.statement_spans.len(), cx.current_scope_id()));
    }

    fn enter_statement(&mut self, node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        let listed = self
            .list_marks
            .last()
            .is_some_and(|(member, _)| *member == self.statement_spans.len());
        if listed {
            order::first_calls(node, &mut self.first_calls);
        }
        self.statement_spans.push(node.span());
    }

    fn exit_statement(&mut self, _node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        self.statement_spans.pop();
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        common::convert(self, expr, cx);
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        self.list_marks.pop();
        self.drain_hoists(node, cx);
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.land_hoists(&mut program.body, cx);
        self.report(cx);
        common::apply_renames(program, cx);
    }
}
