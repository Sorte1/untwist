use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use std::cell::Cell;

use untwist_core::semantic::{observers, Observers};

use super::roles::KEY_DEPTH;
use super::sites::{Forward, Sites};

pub(super) const TEXT_DEPTH: u8 = 4;

impl<'a> Sites<'_, 'a> {
    pub(super) fn reach_forwarded(&mut self, call: &CallExpression<'a>, forward: Forward) {
        if !self.watching_text(call.span) {
            return;
        }
        let reached = (forward.call && self.texts_reach(call.arguments.iter().skip(1)))
            || ((forward.apply || forward.reflect) && {
                let listed = if forward.reflect { 2 } else { 1 };
                self.list_reaches(call.arguments.get(listed))
            });
        if reached {
            self.hit(call.span);
        }
    }

    fn watching_text(&self, span: Span) -> bool {
        observers() != Observers::None && self.watching(span)
    }

    fn list_reaches(&self, list: Option<&Argument<'a>>) -> bool {
        let Some(list) = list else {
            return false;
        };
        let Some(Expression::ArrayExpression(array)) =
            list.as_expression().map(Expression::without_parentheses)
        else {
            return true;
        };
        array.elements.iter().any(|element| {
            element
                .as_expression()
                .is_none_or(|expr| self.text_reaches(expr))
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

    pub(super) fn reach_text(&mut self, span: Span, arguments: &[Argument<'a>]) {
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
        if self.texts_reach(arguments.iter()) {
            self.hit(span);
        }
    }

    pub(super) fn reach_template_text(&mut self, span: Span, text: Option<&str>) {
        if self.watching_text(span) && text.is_none_or(|text| self.names_reach(text)) {
            self.hit(span);
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
            return names_reach_text(text, watched, self.watch.depth);
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
                    .is_some_and(|name| !CONVERSION_KEYS.contains(&name.as_ref()))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn names_reach_at_once(text: &str, names: &[&str], depth: u8) -> bool {
    depth == 0 || crate::proof::names_a_token(text, names)
}

pub(super) fn names_reach_text(text: &str, watched: (&[&str], &[&str]), depth: u8) -> bool {
    if names_reach_at_once(text, watched.0, depth) {
        return true;
    }
    let wrapped = format!("(function () {{\n{text}\n}});");
    [text, wrapped.as_str()]
        .iter()
        .find_map(|form| parsed_reaches(form, watched, depth - 1))
        .unwrap_or(false)
}

fn parsed_reaches(form: &str, watched: (&[&str], &[&str]), depth: u8) -> Option<bool> {
    let names = watched.0;
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
        judge_text(program, named.plain, watched, depth)
    });
    match judged {
        None => Some(true),
        Some(None) => None,
        Some(Some((reached, pending))) => Some(
            reached
                || pending
                    .iter()
                    .any(|text| names_reach_text(text, watched, depth)),
        ),
    }
}

type Judged = (bool, Vec<String>);

fn judge_text<'a>(
    program: &'a Program<'a>,
    plain: bool,
    watched: (&[&str], &[&str]),
    depth: u8,
) -> Judged {
    if !plain {
        let sem = untwist_core::semantic::build_scoping(program);
        return super::text_reach(program, &sem.scoping, watched, depth);
    }
    let judged = super::text_reach(program, &root_only_scoping(program), watched, depth);
    compare_with_built(program, &judged, watched, depth);
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
    depth: u8,
) {
    if COMPARED.with(Cell::get).is_none() {
        return;
    }
    let sem = untwist_core::semantic::build_scoping(program);
    let built = super::text_reach(program, &sem.scoping, watched, depth);
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
        self.check(&it.value);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        if let Some(cooked) = &it.value.cooked {
            self.check(cooked);
        }
        walk_js::walk_template_element(self, it);
    }
}
