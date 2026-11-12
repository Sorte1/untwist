use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::ast_util::edit::ExprEdits;
use untwist_core::ast_util::global::is_global_reference;
use untwist_core::ast_util::inline::{
    inline_function_call, inline_spread_forward, is_plain_function, param_names, rest_param,
    returned_expression,
};
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::{
    identifier_name, literal_member_parts, member_parts, prop_name,
};
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, NoteKind, Pass, RepeatedSites,
};

use std::borrow::Cow;

use crate::notes;
use crate::object_props::ambiguous_key;
use crate::wrapper_calls::arguments_are_substitutable;

mod dead;
mod declared;
mod hoisted;
mod keys;
mod nested;
mod sequenced;

pub use declared::DeclaredObject;

use declared::inline_declared_objects;
use keys::fold_key_assignments;

pub const PASS: &str = "control-flow-object";

const INERT_BINDING: SymbolFlags = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);

pub(super) enum Refusal {
    Shape,
    LogicalArgument,
}

pub(super) enum SiteSafety {
    Safe,
    Refused(Refusal),
}

pub fn is_cff_key(name: &str) -> bool {
    name.len() == 5 && name.chars().all(|ch| ch.is_ascii_alphabetic())
}

pub enum CffValue<'k, 'a> {
    Text(&'k StringLiteral<'a>),
    Function(&'k Function<'a>),
}

fn binary_operands<'k, 'a>(
    function: &'k Function<'a>,
    names: &[&str],
) -> Option<(&'k str, &'k str)> {
    if names.len() != 2 {
        return None;
    }
    let (left, right) = match returned_expression(function)? {
        Expression::BinaryExpression(binary) => (&binary.left, &binary.right),
        Expression::LogicalExpression(logical) => (&logical.left, &logical.right),
        _ => return None,
    };
    Some((identifier_name(left)?, identifier_name(right)?))
}

fn binary_pair(function: &Function<'_>, names: &[&str]) -> bool {
    let Some((left, right)) = binary_operands(function, names) else {
        return false;
    };
    (left == names[0] && right == names[1]) || (left == names[1] && right == names[0])
}

fn binary_operator_pair_in_order(function: &Function<'_>, names: &[&str]) -> bool {
    if !matches!(
        returned_expression(function),
        Some(Expression::BinaryExpression(_))
    ) {
        return false;
    }
    binary_operands(function, names)
        .is_some_and(|(left, right)| left == names[0] && right == names[1])
}

fn forwards_a_binary_pair(function: &Function<'_>) -> bool {
    if function.params.rest.is_some() {
        return false;
    }
    param_names(function).is_some_and(|names| {
        names_are_distinct(&names) && binary_operator_pair_in_order(function, &names)
    })
}

fn dispatch_call(function: &Function<'_>, names: &[&str]) -> bool {
    let Some(returned) = returned_expression(function) else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(returned) else {
        return false;
    };
    if identifier_name(callee) != names.first().copied() {
        return false;
    }
    if arguments.len() + 1 != names.len() {
        return false;
    }
    arguments
        .iter()
        .zip(names.iter().skip(1))
        .all(|(argument, name)| {
            argument
                .as_expression()
                .and_then(identifier_name)
                .is_some_and(|actual| actual == *name)
        })
}

fn spread_call(function: &Function<'_>) -> bool {
    let Some((callee, rest)) = rest_param(function) else {
        return false;
    };
    let Some(returned) = returned_expression(function) else {
        return false;
    };
    let Some((actual_callee, arguments)) = call_parts(returned) else {
        return false;
    };
    if identifier_name(actual_callee) != Some(callee) || arguments.len() != 1 {
        return false;
    }
    let Some(Argument::SpreadElement(spread)) = arguments.first() else {
        return false;
    };
    identifier_name(&spread.argument) == Some(rest)
}

fn names_are_distinct(names: &[&str]) -> bool {
    for (index, name) in names.iter().enumerate() {
        if names[..index].contains(name) {
            return false;
        }
    }
    true
}

fn cff_value<'k, 'a>(expr: &'k Expression<'a>) -> Option<CffValue<'k, 'a>> {
    match expr {
        Expression::StringLiteral(literal) => Some(CffValue::Text(literal)),
        Expression::FunctionExpression(function) => {
            if !is_plain_function(function) {
                return None;
            }
            if spread_call(function) {
                return Some(CffValue::Function(function));
            }
            let names = param_names(function)?;
            if !names_are_distinct(&names) {
                return None;
            }
            if binary_pair(function, &names) || dispatch_call(function, &names) {
                Some(CffValue::Function(function))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn cff_props<'k, 'a>(
    expr: &'k Expression<'a>,
    extended: bool,
) -> Option<Vec<(Cow<'k, str>, CffValue<'k, 'a>)>> {
    let Expression::ObjectExpression(object) = expr else {
        return None;
    };
    if ambiguous_key(expr).is_some() {
        return None;
    }
    let mut props = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.method || property.kind != PropertyKind::Init {
            return None;
        }
        if property.computed && !matches!(property.key, PropertyKey::StringLiteral(_)) {
            return None;
        }
        let name = prop_name(&property.key)?;
        props.push((name, cff_value(&property.value)?));
    }
    if props.is_empty() || !accepted_keys(&props, extended) {
        return None;
    }
    Some(props)
}

pub(crate) fn is_dispatch_object(props: &[(Cow<'_, str>, CffValue<'_, '_>)]) -> bool {
    if props.iter().all(|(key, _)| is_cff_key(key)) {
        return true;
    }
    props.len() >= 2
        && props
            .iter()
            .any(|(_, value)| matches!(value, CffValue::Function(_)))
}

fn accepted_keys(props: &[(Cow<'_, str>, CffValue<'_, '_>)], extended: bool) -> bool {
    if extended {
        return is_dispatch_object(props);
    }
    props.iter().all(|(key, _)| is_cff_key(key))
}

pub(crate) fn non_constant_key(expr: &Expression<'_>) -> Option<Span> {
    let Expression::ObjectExpression(object) = expr else {
        return None;
    };
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        let constant = !property.computed || matches!(property.key, PropertyKey::StringLiteral(_));
        (!constant).then(|| property.key.span())
    })
}

fn near_cff_miss(expr: &Expression<'_>) -> bool {
    let Expression::ObjectExpression(object) = expr else {
        return false;
    };
    let mut saw_non_constant_key = false;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        if property.method || property.kind != PropertyKind::Init {
            return false;
        }
        if cff_value(&property.value).is_none() {
            return false;
        }
        let constant = !property.computed || matches!(property.key, PropertyKey::StringLiteral(_));
        if constant {
            let Some(name) = prop_name(&property.key) else {
                return false;
            };
            if !is_cff_key(&name) {
                return false;
            }
        } else {
            saw_non_constant_key = true;
        }
    }
    saw_non_constant_key
}

fn call_binds_a_receiver(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ComputedMemberExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::PrivateFieldExpression(_)
            | Expression::ChainExpression(_)
            | Expression::TSNonNullExpression(_)
    )
}

fn forwarded_callee_is_safe(arguments: &AVec<'_, Argument<'_>>) -> bool {
    arguments
        .first()
        .and_then(Argument::as_expression)
        .is_some_and(|callee| !call_binds_a_receiver(callee))
}

fn moves_every_argument_once_in_order(
    function: &Function<'_>,
    arguments: &AVec<'_, Argument<'_>>,
) -> bool {
    let Some(names) = param_names(function) else {
        return false;
    };
    if !names_are_distinct(&names) {
        return false;
    }
    if dispatch_call(function, &names) {
        return forwarded_callee_is_safe(arguments);
    }
    binary_operator_pair_in_order(function, &names)
}

fn returns_a_logical(function: &Function<'_>) -> bool {
    matches!(
        returned_expression(function),
        Some(Expression::LogicalExpression(_))
    )
}

fn is_primitive_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}

fn is_signed_numeric_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
            ) && is_signed_numeric_literal(&unary.argument)
        }
        _ => false,
    }
}

fn is_literal_only(expr: &Expression<'_>) -> bool {
    let inner = expr.without_parentheses();
    if is_primitive_literal(inner) {
        return true;
    }
    match inner {
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::LogicalNot => is_literal_only(&unary.argument),
            UnaryOperator::Void => matches!(unary.argument, Expression::NumericLiteral(_)),
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot => {
                is_signed_numeric_literal(&unary.argument)
            }
            UnaryOperator::Typeof | UnaryOperator::Delete => false,
        },
        Expression::BinaryExpression(binary) => {
            !binary.operator.is_relational()
                && is_literal_only(&binary.left)
                && is_literal_only(&binary.right)
        }
        _ => false,
    }
}

fn identifier_cannot_throw(id: &IdentifierReference<'_>, cx: &Cx<'_, '_>) -> bool {
    let Some(reference) = id.reference_id.get() else {
        return false;
    };
    let scoping = cx.scoping();
    match scoping.get_reference(reference).symbol_id() {
        Some(symbol) => {
            let flags = scoping.symbol_flags(symbol);
            let read_before_declared = !flags.contains(SymbolFlags::Function)
                && id.span.start < scoping.symbol_span(symbol).start;
            flags.intersects(INERT_BINDING) && !read_before_declared
        }
        None => id.name.as_str() == "undefined" && is_global_reference(scoping, id),
    }
}

fn is_inert_operand(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    let inner = expr.without_parentheses();
    if is_literal_only(inner) {
        return true;
    }
    match inner {
        Expression::Identifier(id) => identifier_cannot_throw(id, cx),
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot && is_inert_operand(&unary.argument, cx)
        }
        Expression::BinaryExpression(binary) => {
            matches!(
                binary.operator,
                BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
            ) && is_inert_operand(&binary.left, cx)
                && is_inert_operand(&binary.right, cx)
        }
        _ => false,
    }
}

fn is_inert_argument(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    match expr.without_parentheses() {
        Expression::BigIntLiteral(_) => true,
        inner => is_inert_operand(inner, cx),
    }
}

fn logical_site_is_safe(arguments: &AVec<'_, Argument<'_>>, cx: &Cx<'_, '_>) -> bool {
    arguments.iter().all(|argument| {
        argument
            .as_expression()
            .is_some_and(|expr| is_inert_argument(expr, cx))
    })
}

fn reference_route(function: &Function<'_>, arguments: &AVec<'_, Argument<'_>>) -> bool {
    moves_every_argument_once_in_order(function, arguments)
        || arguments_are_substitutable(arguments)
}

fn call_site_is_safe(
    function: &Function<'_>,
    arguments: &AVec<'_, Argument<'_>>,
    cx: &Cx<'_, '_>,
) -> SiteSafety {
    if function.params.rest.is_some() {
        return shape_verdict(forwarded_callee_is_safe(arguments));
    }
    if arguments.len() > function.params.items.len() {
        return SiteSafety::Refused(Refusal::Shape);
    }
    if !cx.opts.extended {
        return shape_verdict(reference_route(function, arguments));
    }
    if returns_a_logical(function) {
        return match logical_site_is_safe(arguments, cx) {
            true => SiteSafety::Safe,
            false => SiteSafety::Refused(Refusal::LogicalArgument),
        };
    }
    shape_verdict(reference_route(function, arguments))
}

fn inline_site<'p, 'a>(
    function: &Function<'p>,
    arguments: &AVec<'p, Argument<'p>>,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let inlined = inline_function_call(function, arguments, cx);
    let inlined = match inlined {
        None if cx.opts.unsafe_transforms => inline_spread_forward(function, arguments, cx),
        inlined => inlined,
    };
    inlined.filter(|expr| !is_direct_eval(expr))
}

fn is_direct_eval(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    matches!(call.callee.without_parentheses(), Expression::Identifier(id) if id.name == "eval")
}

fn shape_verdict(safe: bool) -> SiteSafety {
    match safe {
        true => SiteSafety::Safe,
        false => SiteSafety::Refused(Refusal::Shape),
    }
}

pub(super) fn emit_logical_refusals(sites: RepeatedSites, cx: &mut Cx<'_, '_>) {
    sites.emit(
        NoteKind::Refusal,
        |_| notes::LOGICAL_FORWARDER_ARGUMENT.to_string(),
        |total, _| notes::logical_forwarder_arguments(total),
        cx,
    );
}

pub(crate) fn single_declarator<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<(&'k BindingIdentifier<'a>, Option<&'k Expression<'a>>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((&**id, declarator.init.as_ref()))
}

struct ObjectDeclaration<'k> {
    name: &'k str,
    symbol: SymbolId,
    object: Span,
    keys: Vec<Cow<'k, str>>,
}

fn object_declaration<'k>(
    stmt: &'k Statement<'_>,
    extended: bool,
) -> Option<ObjectDeclaration<'k>> {
    let (id, init) = single_declarator(stmt)?;
    let init = init?;
    let Expression::ObjectExpression(object) = init else {
        return None;
    };
    let keys = if object.properties.is_empty() {
        Vec::new()
    } else {
        cff_props(init, extended)?
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    };
    Some(ObjectDeclaration {
        name: id.name.as_str(),
        symbol: id.symbol_id.get()?,
        object: init.span(),
        keys,
    })
}

#[derive(Default)]
pub struct ControlFlowObject<'a> {
    edits: ExprEdits<'a>,
    logical_refusals: RepeatedSites,
    callee_objects: bool,
    open_sites: Vec<Span>,
    refused_nest: Option<Span>,
}

pub(super) const MAX_NEST: usize = 256;

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub(super) fn nested_past_the_limit(spans: impl Iterator<Item = Span>) -> bool {
    let mut spans: Vec<Span> = spans.collect();
    if spans.len() <= MAX_NEST {
        return false;
    }
    spans.sort_unstable_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    spans.dedup();
    let mut open: Vec<Span> = Vec::new();
    for span in spans {
        while open.last().is_some_and(|outer| !contains(*outer, span)) {
            open.pop();
        }
        open.push(span);
        if open.len() > MAX_NEST {
            return true;
        }
    }
    false
}

impl ControlFlowObject<'_> {
    fn nest_refused(&mut self, span: Span, cx: &mut Cx<'_, '_>) -> bool {
        if self.refused_nest.is_some_and(|root| contains(root, span)) {
            return true;
        }
        self.open_sites.retain(|outer| contains(*outer, span));
        self.open_sites.push(span);
        if self.open_sites.len() <= MAX_NEST {
            return false;
        }
        let root = self.open_sites[0];
        for site in self.open_sites.drain(..) {
            self.edits.remove(site);
        }
        self.refused_nest = Some(root);
        cx.note(notes::CONTROL_FLOW_NEST_TOO_DEEP, Some(root));
        true
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(ControlFlowObject::default())
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::CallExpression,
    ExpressionKind::StaticMemberExpression,
    ExpressionKind::ComputedMemberExpression,
]);

impl<'a> Pass<'a> for ControlFlowObject<'a> {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::EXIT_STATEMENTS
            | Interest::ENTER_EXPRESSION
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_PROGRAM)
            .on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.open_sites.clear();
        self.refused_nest = None;
        let applied = inline_declared_objects(program, cx);
        cx.changed_by(applied);
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        let folded = fold_key_assignments(stmts, cx);
        cx.changed_by(folded);
        let mut declared = hoisted::declare_assigned_objects(stmts, cx);
        if declared == 0 && self.callee_objects {
            declared = hoisted::declare_callee_objects(stmts, cx);
        }
        if declared == 0 {
            declared = sequenced::declare_sequenced_objects(stmts, cx);
        }
        cx.changed_by(declared);
    }

    fn enter_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Some((callee, arguments)) = call_parts(expr) else {
            return;
        };
        let Some((object, name)) = member_parts(callee) else {
            return;
        };
        if !self.callee_objects
            && matches!(
                object.without_parentheses(),
                Expression::AssignmentExpression(_)
            )
        {
            if let Expression::CallExpression(call) = &*expr {
                self.callee_objects = hoisted::assigns_its_callee_object(call);
            }
        }
        let Some(props) = cff_props(object, cx.opts.extended) else {
            return;
        };
        let Some((_, CffValue::Function(function))) = props.iter().find(|(key, _)| *key == name)
        else {
            return;
        };
        if self.nest_refused(expr.span(), cx) {
            return;
        }
        match call_site_is_safe(function, arguments, cx) {
            SiteSafety::Safe => {}
            SiteSafety::Refused(Refusal::LogicalArgument) => {
                self.logical_refusals.record(expr.span(), &name);
                return;
            }
            SiteSafety::Refused(Refusal::Shape) => return,
        }
        let Some(inlined) = inline_site(function, arguments, &*cx) else {
            return;
        };
        self.edits.insert(expr.span(), inlined);
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Some((object, name)) = literal_member_parts(expr) else {
            return;
        };
        let Some(props) = cff_props(object, cx.opts.extended) else {
            if near_cff_miss(object) {
                if let Some(span) = non_constant_key(object) {
                    cx.note(notes::CONTROL_FLOW_NON_CONSTANT_KEY, Some(span));
                }
            }
            return;
        };
        let Some((_, value)) = props.iter().find(|(key, _)| *key == name) else {
            return;
        };
        if let CffValue::Function(function) = value {
            if function.id.is_none() {
                return;
            }
        }
        let replacement = match value {
            CffValue::Text(literal) => Expression::StringLiteral(oxc_allocator::Box::new_in(
                (*literal).clone_in(cx.allocator()),
                &*cx,
            )),
            CffValue::Function(function) => Expression::FunctionExpression(
                oxc_allocator::Box::new_in((*function).clone_in(cx.allocator()), &*cx),
            ),
        };
        self.edits.insert(expr.span(), replacement);
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let mut edits = std::mem::take(&mut self.edits);
        if edits.forget_renamed_since_refresh(program) > 0 {
            cx.note(notes::CONTROL_FLOW_RENAMED_BEFORE_INSERTION, None);
        }
        let wanted = edits.len();
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
        if applied != wanted {
            cx.note(
                notes::planned_and_applied_edits(PASS, wanted, applied),
                None,
            );
        }
        emit_logical_refusals(std::mem::take(&mut self.logical_refusals), cx);
    }
}
