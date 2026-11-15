use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use std::cell::Cell;
use untwist_core::ast_util::js_text::js_text;

use untwist_core::semantic::{observers, Observers};

use super::roles::KEY_DEPTH;
use super::sites::{Forward, Sites};

pub(super) const TEXT_DEPTH: u8 = 4;

impl<'a> Sites<'_, 'a> {
    pub(super) fn reach_forwarded(
        &mut self,
        call: &CallExpression<'a>,
        forward: Forward,
        function: bool,
    ) {
        if !self.watching_text(call.span) {
            return;
        }
        let passed: Vec<Option<&Expression<'a>>> = call
            .arguments
            .iter()
            .skip(1)
            .map(Argument::as_expression)
            .collect();
        let reached = (forward.call
            && match function {
                true => self.joined_reaches(&passed),
                false => self.texts_reach(call.arguments.iter().skip(1)),
            })
            || ((forward.apply || forward.reflect) && {
                let listed = if forward.reflect { 2 } else { 1 };
                self.list_reaches(call.arguments.get(listed), function)
            });
        if reached {
            self.hit_text(call.span);
        }
    }

    pub(super) fn judges_every_call_as_text(&self) -> bool {
        observers() != Observers::None || self.named_runner_escaped
    }

    fn watching_text(&self, span: Span) -> bool {
        (observers() != Observers::None || self.watch.texts_in_every_lane) && self.watching(span)
    }

    pub(super) fn mark_opaque_text(&mut self, span: Span) {
        if self.watching_text(span) {
            self.hit_text(span);
        }
        self.mark_code(span);
    }

    pub(super) fn text_watched(&self, span: Span) -> bool {
        self.watching_text(span)
    }

    pub(super) fn text_hits(&self, span: Span, text: &Expression<'a>) -> bool {
        self.watching_text(span) && self.text_reaches(text)
    }

    fn list_reaches(&self, list: Option<&Argument<'a>>, function: bool) -> bool {
        let Some(list) = list else {
            return false;
        };
        let Some(Expression::ArrayExpression(array)) =
            list.as_expression().map(Expression::without_parentheses)
        else {
            return true;
        };
        let elements: Vec<Option<&Expression<'a>>> = array
            .elements
            .iter()
            .map(ArrayExpressionElement::as_expression)
            .collect();
        match function {
            true => self.joined_reaches(&elements),
            false => elements
                .iter()
                .any(|element| element.is_none_or(|expr| self.text_reaches(expr))),
        }
    }

    fn joined_reaches(&self, texts: &[Option<&Expression<'a>>]) -> bool {
        if texts.is_empty() {
            return false;
        }
        let resolved: Option<Vec<String>> = texts
            .iter()
            .map(|text| text.and_then(|expr| self.text_value(expr, KEY_DEPTH)))
            .collect();
        let Some(resolved) = resolved else {
            return true;
        };
        let Some((body, params)) = resolved.split_last() else {
            return false;
        };
        let params = params.join(",");
        FUNCTION_HEADS.iter().any(|head| {
            let source = format!("({head} ({params}\n) {{\n{body}\n}});");
            self.names_reach(&source)
        })
    }

    fn texts_reach<'k>(&self, mut arguments: impl Iterator<Item = &'k Argument<'a>>) -> bool
    where
        'a: 'k,
    {
        arguments.any(|argument| {
            argument
                .as_expression()
                .is_none_or(|expr| self.text_reaches(expr))
        })
    }

    pub(super) fn reach_unused_text(&mut self, span: Span, arguments: &[Argument<'a>]) {
        self.result_unused.set(true);
        self.reach_text(span, arguments, false);
        self.result_unused.set(false);
    }

    pub(super) fn reach_text(&mut self, span: Span, arguments: &[Argument<'a>], function: bool) {
        if !self.watching_text(span) {
            return;
        }
        let first_fn = arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|first| self.fresh.is_user_function(first));
        if first_fn {
            return;
        }
        let texts: Vec<Option<&Expression<'a>>> =
            arguments.iter().map(Argument::as_expression).collect();
        let reached = match function {
            true => self.joined_reaches(&texts),
            false => self.texts_reach(arguments.iter()),
        };
        if reached {
            self.hit_text(span);
        }
    }

    pub(super) fn reach_template_text(&mut self, span: Span, text: Option<&str>) {
        if self.watching_text(span) && text.is_none_or(|text| self.names_reach(text)) {
            self.hit_text(span);
        }
    }

    fn text_reaches(&self, expr: &Expression<'a>) -> bool {
        let inner = expr.without_parentheses();
        let inert = matches!(
            inner,
            Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
        ) || self.fresh.is_numeric(inner);
        !inert
            && self
                .text_value(inner, KEY_DEPTH)
                .is_none_or(|text| self.names_reach(&text))
    }

    fn names_reach(&self, text: &str) -> bool {
        let watched = (self.watch.names, self.watch.props);
        if !self.watch.defer {
            let judged = (
                self.watch.depth,
                self.watch.runner_texts && !self.result_unused.get(),
            );
            return names_reach_text(text, watched, judged);
        }
        if names_reach_at_once(text, watched.0, self.watch.depth) {
            return true;
        }
        self.pending_texts.borrow_mut().push(text.to_string());
        false
    }

    pub(super) fn text_value(&self, expr: &Expression<'a>, depth: u8) -> Option<String> {
        let next = depth.checked_sub(1)?;
        match expr.without_parentheses() {
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                let left = self.text_value(&b.left, next)?;
                Some(left + &self.text_value(&b.right, next)?)
            }
            Expression::ComputedMemberExpression(member) => self
                .element_text(member, next)
                .or_else(|| self.watched_text(expr).map(str::to_string))
                .or_else(|| self.str_value(expr, depth)),
            Expression::Identifier(id) => match self.substituted(id) {
                Some(arg) => self.text_value(arg, next),
                None => self.str_value(expr, depth),
            },
            Expression::ObjectExpression(object) if plain_object(object) => {
                Some("[object Object]".to_string())
            }
            other => self.str_value(other, depth),
        }
    }

    fn element_text(&self, member: &ComputedMemberExpression<'a>, depth: u8) -> Option<String> {
        let Expression::Identifier(id) = member.object.without_parentheses() else {
            return None;
        };
        let Expression::NumericLiteral(index) = member.expression.without_parentheses() else {
            return None;
        };
        let element = self.fresh.sealed_element(id, index.value)?;
        self.text_value(element, depth)
    }
}

const CONVERSION_KEYS: [&str; 4] = ["toString", "valueOf", "__proto__", "constructor"];

fn plain_object(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|property| match property {
        ObjectPropertyKind::ObjectProperty(p) => {
            !p.computed
                && p.key
                    .static_name()
                    .and_then(|name| name.into_cow_str())
                    .is_some_and(|name| !CONVERSION_KEYS.contains(&name.as_ref()))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn names_reach_at_once(text: &str, names: &[&str], depth: u8) -> bool {
    depth == 0 || crate::proof::names_a_token(text, names)
}

type TextJudgement = (u8, bool);

pub(super) fn names_reach_text(
    text: &str,
    watched: (&[&str], &[&str]),
    (depth, runner_texts): TextJudgement,
) -> bool {
    if names_reach_at_once(text, watched.0, depth) {
        return true;
    }
    let mut forms = vec![text.to_string()];
    forms.extend(
        FUNCTION_HEADS
            .iter()
            .map(|head| format!("({head} () {{\n{text}\n}});")),
    );
    forms
        .iter()
        .filter_map(|form| parsed_reaches(form, watched, (depth - 1, runner_texts)))
        .any(|reached| reached)
}

fn parsed_reaches(
    form: &str,
    watched: (&[&str], &[&str]),
    judgement: TextJudgement,
) -> Option<bool> {
    let names = watched.0;
    if crate::proof::with_parsed_text(form, |_| ())
        .flatten()
        .is_some()
        && has_early_errors(form)
    {
        return Some(true);
    }
    let judged = crate::proof::with_parsed_text(form, |program| {
        let mut named = Named {
            names,
            found: false,
            plain: true,
        };
        named.visit_program(program);
        if named.found {
            return (true, Vec::new());
        }
        let judged = judge_text(program, named.plain, watched, judgement);
        match judgement.1 && may_hand_out_a_runner(program) {
            true => (true, Vec::new()),
            false => judged,
        }
    });
    match judged {
        None => Some(true),
        Some(None) => None,
        Some(Some((reached, pending))) => Some(
            reached
                || pending
                    .iter()
                    .any(|text| names_reach_text(text, watched, judgement)),
        ),
    }
}

const FUNCTION_HEADS: [&str; 4] = ["function", "async function", "function*", "async function*"];

fn has_early_errors(form: &str) -> bool {
    crate::proof::with_parsed_text(form, |program| {
        !untwist_core::parse::early_errors(program).is_empty()
    })
    .flatten()
    .unwrap_or(false)
}

fn may_hand_out_a_runner(program: &Program<'_>) -> bool {
    super::inert_text::may_hand_back_a_runner(program)
        && !super::global_text::returns_the_global_object(program)
}

type Judged = (bool, Vec<String>);

fn judge_text<'a>(
    program: &'a Program<'a>,
    plain: bool,
    watched: (&[&str], &[&str]),
    judgement: TextJudgement,
) -> Judged {
    if !plain {
        let sem = untwist_core::semantic::build_scoping(program);
        return super::text_reach(program, &sem.scoping, watched, judgement);
    }
    let judged = super::text_reach(program, &root_only_scoping(program), watched, judgement);
    compare_with_built(program, &judged, watched, judgement);
    judged
}

fn root_only_scoping(program: &Program<'_>) -> Scoping {
    let mut flags = ScopeFlags::Top;
    if program.source_type.is_strict() || program.has_use_strict_directive() {
        flags |= ScopeFlags::StrictMode;
    }
    let mut scoping = Scoping::default();
    scoping.add_scope(None, NodeId::ROOT, flags);
    scoping
}

thread_local! {
    static COMPARED: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

#[cfg(feature = "test-hooks")]
pub fn comparing_text_scopings<R>(run: impl FnOnce() -> R) -> (R, usize, usize) {
    let outer = COMPARED.with(|cell| cell.replace(Some((0, 0))));
    let result = run();
    let (compared, differing) = COMPARED
        .with(|cell| cell.replace(outer))
        .unwrap_or_default();
    (result, compared, differing)
}

fn compare_with_built<'a>(
    program: &'a Program<'a>,
    judged: &Judged,
    watched: (&[&str], &[&str]),
    judgement: TextJudgement,
) {
    if COMPARED.with(Cell::get).is_none() {
        return;
    }
    let sem = untwist_core::semantic::build_scoping(program);
    let built = super::text_reach(program, &sem.scoping, watched, judgement);
    let differs = usize::from(built != *judged);
    COMPARED.with(|cell| {
        if let Some((compared, differing)) = cell.get() {
            cell.set(Some((compared + 1, differing + differs)));
        }
    });
}

struct Named<'n> {
    names: &'n [&'n str],
    found: bool,
    plain: bool,
}

impl Named<'_> {
    fn check(&mut self, name: &str) {
        self.found |= self.names.contains(&name);
    }
}

impl<'a> VisitJs<'a> for Named<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.check(&it.name);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.plain = false;
        self.check(&it.name);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if matches!(it, SimpleAssignmentTarget::AssignmentTargetIdentifier(_)) {
            self.plain = false;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.plain = false;
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.check(&it.name);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.check(&js_text(it.value));
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        if let Some(cooked) = it.value.cooked {
            self.check(&js_text(cooked));
        }
        walk_js::walk_template_element(self, it);
    }
}
