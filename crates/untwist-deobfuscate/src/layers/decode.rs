use oxc_ast::ast::{
    Argument, CallExpression, Expression, IdentifierReference, Program, StringLiteral,
};
use oxc_ast_visit::Visit;
use untwist_core::ast_util::js_text::js_text;
use untwist_core::semantic::{is_foreign_global, program_is_strict};
use untwist_eval::budget::STRING_LEN_CAP;
use untwist_eval::fold::{fold_layer_text, LayerText};

use oxc_semantic::Scoping;

use super::model::{DecodeCx, Layer, LayerForm, Refusal, Refused};
use super::probe::Probe;

pub type Text = Result<(String, u64, u64), Refusal>;

pub fn refuse(form: LayerForm, reason: Refusal) -> Result<Layer, Refused> {
    Err(Refused { form, reason })
}

pub fn outer_call<'p, 'a>(probe: &Probe<'p, 'a>) -> Option<&'p CallExpression<'a>> {
    match probe.expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        _ => None,
    }
}

pub fn plain_arguments<'p, 'a>(arguments: &'p [Argument<'a>]) -> Option<Vec<&'p Expression<'a>>> {
    arguments.iter().map(Argument::as_expression).collect()
}

pub fn sloppy_script(program: &Program<'_>) -> bool {
    !program_is_strict(program) && program.hashbang.is_none()
}

pub fn fold_text(expr: &Expression<'_>, cx: &DecodeCx) -> Text {
    let folded = fold_layer_text(expr, cx.fuel, cx.heap);
    match folded.text {
        LayerText::Text(text) => Ok((text, folded.steps, folded.heap)),
        LayerText::NotAString => Err(Refusal::NotAString),
        LayerText::Bailed => Err(Refusal::Bailed),
        LayerText::Fuel => Err(Refusal::Fuel),
    }
}

pub fn fold_texts(
    exprs: &[&Expression<'_>],
    cx: &DecodeCx,
) -> Result<(Vec<String>, u64, u64), Refusal> {
    let mut texts = Vec::with_capacity(exprs.len());
    let (mut steps, mut heap) = (0_u64, 0_u64);
    for expr in exprs {
        let left = DecodeCx {
            fuel: cx.fuel.saturating_sub(steps),
            heap: cx.heap.saturating_sub(heap),
            ..*cx
        };
        let (text, spent, charged) = fold_text(expr, &left)?;
        texts.push(text);
        steps = steps.saturating_add(spent);
        heap = heap.max(charged);
    }
    Ok((texts, steps, heap))
}

#[derive(Default)]
pub struct FreeNames<'a> {
    pub names: Vec<&'a str>,
    pub over_cap: bool,
}

impl<'a> Visit<'a> for FreeNames<'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if !self.names.contains(&name) {
            self.names.push(name);
        }
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        let text = js_text(it.value);
        self.over_cap |=
            text.len() > STRING_LEN_CAP && text.encode_utf16().count() > STRING_LEN_CAP;
    }
}

pub fn argument_refusal<'a>(
    scoping: &Scoping,
    roots: &[&'a str],
    exprs: &[&Expression<'a>],
) -> Option<Refusal> {
    let mut free = FreeNames::default();
    free.names.extend_from_slice(roots);
    for expr in exprs {
        free.visit_expression(expr);
    }
    if !names_hold(scoping, &free.names) {
        return Some(Refusal::Shape);
    }
    free.over_cap.then_some(Refusal::TooLarge)
}

pub fn names_hold(scoping: &Scoping, names: &[&str]) -> bool {
    if names.iter().any(|name| is_foreign_global(name)) {
        return false;
    }
    if scoping.symbol_names().any(|name| names.contains(&name)) {
        return false;
    }
    let unresolved = scoping.root_unresolved_references();
    names.iter().all(|name| {
        unresolved.get(*name).is_none_or(|references| {
            references
                .iter()
                .all(|reference| !scoping.get_reference(*reference).is_write())
        })
    })
}
