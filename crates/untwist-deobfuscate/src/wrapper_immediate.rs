use oxc_allocator::{GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::edit::ExprEdits;
use untwist_core::ast_util::inline::{inline_function_call, param_names, returned_expression};
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::Cx;

use crate::call_checks::{call_checks_apply, is_plain_function};
use crate::primitive_names::PrimitiveNames;
use crate::stamp::decoder_index;
use crate::wrapper_args::{body_uses, Kind, SiteView};
use crate::wrapper_calls::{
    arguments_are_substitutable, arguments_hold_a_scope, body_is_self_contained, uses_any_param,
};
use crate::wrapper_sites::called_expression;

const IMMEDIATE_PARAMETER_FLOOR: usize = 2;

type Immediate<'k, 'p> = (&'k Function<'p>, &'k AVec<'p, Argument<'p>>, bool);

fn outer_callee(
    callee: &Expression<'_>,
    function: &Function<'_>,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let Expression::Identifier(id) = callee else {
        return None;
    };
    let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
    (Some(scoping.symbol_scope_id(symbol)) != function.scope_id.get()).then_some(symbol)
}

fn immediate_wrapper<'k, 'p>(
    expr: &'k Expression<'p>,
    scoping: &Scoping,
    checked: bool,
) -> Option<Immediate<'k, 'p>> {
    let call = called_expression(expr)?;
    let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
        return None;
    };
    let names = param_names(function)?;
    let returned = returned_expression(function)?;
    let (callee, arguments) = call_parts(returned)?;
    let to_decoder = identifier_name(callee).and_then(decoder_index).is_some();
    let forwards_to = match (to_decoder, checked) {
        (true, _) => None,
        (false, true) => Some(outer_callee(callee, function, scoping)?),
        (false, false) => return None,
    };
    let wraps = function.id.is_none()
        && !function.r#async
        && !function.generator
        && names.len() >= IMMEDIATE_PARAMETER_FLOOR
        && !arguments.is_empty()
        && uses_any_param(arguments, &names)
        && body_is_self_contained(function, scoping, forwards_to)
        && arguments_are_substitutable(&call.arguments)
        && !arguments_hold_a_scope(&call.arguments);
    wraps.then_some((&**function, &call.arguments, forwards_to.is_some()))
}

struct ImmediateSearch<'s, 'c, 'a, 'x> {
    scoping: &'s Scoping,
    cx: &'c Cx<'a, 'x>,
    checked: bool,
    primitives: PrimitiveNames,
    parameters: usize,
    edits: ExprEdits<'a>,
}

impl ImmediateSearch<'_, '_, '_, '_> {
    fn passes_the_call_checks(&self, (function, arguments, forwards): Immediate<'_, '_>) -> bool {
        if !self.checked {
            return true;
        }
        let view = SiteView::new(self.scoping, &self.primitives);
        let in_parameters = self.parameters > 0;
        let pure = || {
            view.kinds_of(arguments, in_parameters)
                .is_some_and(|kinds| kinds.iter().all(|kind| *kind == Kind::Pure))
        };
        is_plain_function(function)
            && (!forwards || pure())
            && body_uses(function)
                .is_some_and(|uses| view.arguments_fit(&uses, arguments, in_parameters))
    }
}

impl<'p> VisitJs<'p> for ImmediateSearch<'_, '_, '_, '_> {
    fn visit_expression(&mut self, it: &Expression<'p>) {
        let found = immediate_wrapper(it, self.scoping, self.checked)
            .filter(|immediate| self.passes_the_call_checks(*immediate));
        if let Some((function, arguments, _)) = found {
            if let Some(inlined) = inline_function_call(function, arguments, self.cx) {
                if self.edits.insert(it.span(), inlined) {
                    return;
                }
            }
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'p>) {
        self.parameters += 1;
        walk_js::walk_formal_parameters(self, it);
        self.parameters -= 1;
    }
}

pub fn inline_immediate_wrappers<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> usize {
    let edits = {
        let view: &Program<'_> = program;
        let checked = call_checks_apply(cx.opts);
        let primitives = if checked {
            PrimitiveNames::collect(view, cx.scoping())
        } else {
            PrimitiveNames::default()
        };
        let mut search = ImmediateSearch {
            scoping: cx.scoping(),
            cx: &*cx,
            checked,
            primitives,
            parameters: 0,
            edits: ExprEdits::new(),
        };
        search.visit_program(view);
        search.edits
    };
    if edits.is_empty() {
        return 0;
    }
    edits.apply(program, cx.allocator(), cx.scoping_mut())
}
