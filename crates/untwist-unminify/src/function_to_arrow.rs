use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::frame_reads::reads_function_frame;
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Pass, StatementKind, StatementVariants,
};
use untwist_core::semantic::{is_constant_binding, scope_hazard};

use crate::var_to_let::{is_text_read, Frozen};

pub const PASS: &str = "function-to-arrow";

pub const SHAPE_CALLED_BINDING: &str = "called-binding";
pub const SHAPE_CALLBACK_ARGUMENT: &str = "callback-argument";

const OWN_FRAME_NAMES: [&str; 3] = ["arguments", "yield", "await"];

#[derive(Default)]
pub struct FunctionToArrow<'a> {
    active: bool,
    callees: Vec<u64>,
    pending: Vec<u64>,
    frozen: Frozen,
    spare: Option<Expression<'a>>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FunctionToArrow::default())
}

#[cfg(feature = "test-hooks")]
fn note_shape(shape: &'static str) {
    crate::untranspile::shapes::note(PASS, shape);
}

#[cfg(not(feature = "test-hooks"))]
fn note_shape(_shape: &'static str) {}

#[derive(Default)]
struct FrameScan {
    found: bool,
}

impl<'a> VisitJs<'a> for FrameScan {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= OWN_FRAME_NAMES.contains(&it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= OWN_FRAME_NAMES.contains(&it.name.as_str());
    }
}

#[derive(Default)]
struct ParameterNames<'a> {
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for ParameterNames<'a> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.push(it.name.as_str());
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn parameters_are_distinct(params: &FormalParameters<'_>) -> bool {
    let mut scan = ParameterNames::default();
    scan.visit_formal_parameters(params);
    let mut names = scan.names;
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    names.len() == count
}

fn uses_own_frame(function: &Function<'_>) -> bool {
    let mut scan = FrameScan::default();
    scan.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        scan.visit_function_body(body);
    }
    scan.found
}

fn own_name_is_unused(function: &Function<'_>, scoping: &Scoping) -> bool {
    let Some(id) = &function.id else {
        return true;
    };
    id.symbol_id
        .get()
        .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty())
}

fn is_convertible(function: &Function<'_>, scoping: &Scoping) -> bool {
    !function.generator
        && function.body.is_some()
        && function.scope_id.get().is_some()
        && function.this_param.is_none()
        && function.type_parameters.is_none()
        && function.return_type.is_none()
        && own_name_is_unused(function, scoping)
        && parameters_are_distinct(&function.params)
        && !uses_own_frame(function)
        && !is_text_read(function)
}

fn arrow_scope(scoping: &mut Scoping, scope: ScopeId, id: Option<&BindingIdentifier<'_>>) {
    *scoping.scope_flags_mut(scope) |= ScopeFlags::Arrow;
    if let Some(id) = id {
        scoping.remove_binding(scope, id.name);
    }
}

fn into_arrow<'a>(
    mut function: ABox<'a, Function<'a>>,
    cx: &mut Cx<'a, '_>,
) -> Result<ABox<'a, ArrowFunctionExpression<'a>>, ABox<'a, Function<'a>>> {
    let Some(scope) = function.scope_id.get() else {
        return Err(function);
    };
    let Some(body) = function.body.take() else {
        return Err(function);
    };
    arrow_scope(cx.scoping_mut(), scope, function.id.as_ref());
    let function = function.unbox();
    Ok(
        ArrowFunctionExpression::boxed_with_scope_id_and_pure_and_pife(
            function.span,
            function.r#async,
            None,
            function.params,
            None,
            ArrowFunctionBody::FunctionBody(body),
            scope,
            function.pure,
            function.pife,
            &*cx,
        ),
    )
}

fn hole<'a>(spare: &mut Option<Expression<'a>>, cx: &Cx<'a, '_>) -> Expression<'a> {
    spare
        .take()
        .unwrap_or_else(|| Expression::new_null_literal(SPAN, cx))
}

fn arrow_or_back<'a>(
    function: ABox<'a, Function<'a>>,
    cx: &mut Cx<'a, '_>,
) -> (Expression<'a>, bool) {
    match into_arrow(function, cx) {
        Ok(arrow) => (Expression::ArrowFunctionExpression(arrow), true),
        Err(function) => (Expression::FunctionExpression(function), false),
    }
}

fn convert_expression<'a>(
    slot: &mut Expression<'a>,
    spare: &mut Option<Expression<'a>>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    if !matches!(slot, Expression::FunctionExpression(_)) {
        return false;
    }
    let placeholder = hole(spare, cx);
    let Expression::FunctionExpression(function) = std::mem::replace(slot, placeholder) else {
        return false;
    };
    let (converted, done) = arrow_or_back(function, cx);
    *spare = Some(std::mem::replace(slot, converted));
    done
}

fn convert_argument<'a>(
    slot: &mut Argument<'a>,
    spare: &mut Option<Expression<'a>>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    if !matches!(slot, Argument::FunctionExpression(_)) {
        return false;
    }
    let placeholder = Argument::from(hole(spare, cx));
    let Argument::FunctionExpression(function) = std::mem::replace(slot, placeholder) else {
        return false;
    };
    let (converted, done) = arrow_or_back(function, cx);
    if let Argument::NullLiteral(null) = std::mem::replace(slot, Argument::from(converted)) {
        *spare = Some(Expression::NullLiteral(null));
    }
    done
}

fn is_convertible_callback(argument: &Argument<'_>, scoping: &Scoping) -> bool {
    matches!(argument, Argument::FunctionExpression(function)
        if function.id.is_none() && is_convertible(function, scoping))
}

fn receiver_arguments(callee: &Expression<'_>) -> usize {
    let Expression::StaticMemberExpression(member) = callee else {
        return 0;
    };
    match member.property.name.as_str() {
        "call" | "apply" | "bind" => 1,
        "construct" => usize::MAX,
        _ => 0,
    }
}

impl<'a> FunctionToArrow<'a> {
    fn mark_callee(&mut self, call: &CallExpression<'_>) {
        let Expression::Identifier(callee) = &call.callee else {
            return;
        };
        if let Some(reference) = callee.reference_id.get() {
            set_bit(&mut self.callees, reference.index());
        }
    }

    fn is_callee(&self, index: usize) -> bool {
        self.callees
            .get(index / 64)
            .is_some_and(|word| word & (1u64 << (index % 64)) != 0)
    }

    fn only_called(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        is_constant_binding(scoping, symbol)
            && scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .all(|reference| self.is_callee(reference.index()))
    }

    fn is_called_binding(
        &self,
        declarator: &VariableDeclarator<'_>,
        scoping: &Scoping,
        scope: ScopeId,
    ) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        let Some(Expression::FunctionExpression(function)) = &declarator.init else {
            return false;
        };
        id.symbol_id.get().is_some_and(|symbol| {
            scoping.symbol_scope_id(symbol) == scope
                && self.only_called(scoping, symbol)
                && is_convertible(function, scoping)
        })
    }

    fn called_bindings(
        &self,
        stmts: &AVec<'_, Statement<'_>>,
        scoping: &Scoping,
        scope: ScopeId,
    ) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        for (index, stmt) in stmts.iter().enumerate() {
            let Statement::VariableDeclaration(declaration) = stmt else {
                continue;
            };
            for (at, declarator) in declaration.declarations.iter().enumerate() {
                if self.is_called_binding(declarator, scoping, scope) {
                    found.push((index, at));
                }
            }
        }
        found
    }

    #[inline(never)]
    fn convert_called_bindings(
        &mut self,
        stmts: &mut AVec<'a, Statement<'a>>,
        scope: ScopeId,
        cx: &mut Cx<'a, '_>,
    ) {
        for (index, at) in self.called_bindings(stmts, cx.scoping(), scope) {
            let Some(init) = declarator_init(stmts, index, at) else {
                continue;
            };
            if convert_expression(init, &mut self.spare, cx) {
                note_shape(SHAPE_CALLED_BINDING);
                cx.changed();
            }
        }
    }

    #[inline(never)]
    fn convert_callbacks(&mut self, call: &mut CallExpression<'a>, cx: &mut Cx<'a, '_>) {
        if self.frozen.is_open() {
            return;
        }
        let skipped = receiver_arguments(&call.callee);
        for argument in call.arguments.iter_mut().skip(skipped) {
            if !is_convertible_callback(argument, cx.scoping()) {
                continue;
            }
            if convert_argument(argument, &mut self.spare, cx) {
                note_shape(SHAPE_CALLBACK_ARGUMENT);
                cx.changed();
            }
        }
    }
}

fn set_bit(bits: &mut [u64], index: usize) {
    if let Some(word) = bits.get_mut(index / 64) {
        *word |= 1u64 << (index % 64);
    }
}

fn take_bit(bits: &mut [u64], index: usize) -> bool {
    let Some(word) = bits.get_mut(index / 64) else {
        return false;
    };
    let mask = 1u64 << (index % 64);
    let was = *word & mask != 0;
    *word &= !mask;
    was
}

fn declarator_init<'s, 'a>(
    stmts: &'s mut AVec<'a, Statement<'a>>,
    index: usize,
    at: usize,
) -> Option<&'s mut Expression<'a>> {
    let Some(Statement::VariableDeclaration(declaration)) = stmts.get_mut(index) else {
        return None;
    };
    declaration.declarations.get_mut(at)?.init.as_mut()
}

impl<'a> Pass<'a> for FunctionToArrow<'a> {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::ENTER_STATEMENT
            | Interest::ENTER_EXPRESSION
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_FUNCTION
            | Interest::EXIT_STATEMENTS)
            .on_statements(StatementVariants::of(&[StatementKind::FunctionDeclaration]))
            .on_expressions(ExpressionVariants::of(&[
                ExpressionKind::CallExpression,
                ExpressionKind::FunctionExpression,
            ]))
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.callees.clear();
        self.pending.clear();
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let scoping = cx.scoping();
        self.active = scope_hazard(scoping).is_none()
            && (cx.opts.unsafe_transforms || !reads_function_frame(program, scoping));
        self.callees.clear();
        self.pending.clear();
        self.frozen.clear();
        if self.active {
            self.callees
                .resize(scoping.references_len().div_ceil(64), 0);
            self.pending.resize(scoping.scopes_len().div_ceil(64), 0);
        }
    }

    fn enter_statement(&mut self, node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        if let Statement::FunctionDeclaration(function) = node {
            self.frozen.enter(function);
        }
    }

    fn enter_expression(&mut self, node: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        if let Expression::FunctionExpression(function) = node {
            self.frozen.enter(function);
        }
    }

    fn exit_function(&mut self, node: &mut Function<'a>, _cx: &mut Cx<'a, '_>) {
        self.frozen.exit(node);
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if !self.active {
            return;
        }
        match node {
            Expression::CallExpression(call) => {
                self.mark_callee(call);
                if cx.opts.unsafe_transforms {
                    self.convert_callbacks(call, cx);
                }
            }
            Expression::FunctionExpression(_) => {
                set_bit(&mut self.pending, cx.current_scope_id().index());
            }
            _ => {}
        }
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !self.active || !take_bit(&mut self.pending, cx.current_scope_id().index()) {
            return;
        }
        if self.frozen.is_open() {
            return;
        }
        let scope = cx.current_scope_id();
        if scope == cx.scoping().root_scope_id() {
            return;
        }
        self.convert_called_bindings(stmts, scope, cx);
    }
}
