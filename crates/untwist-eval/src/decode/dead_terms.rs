use std::cell::{Cell, RefCell};
use std::rc::Rc;

use oxc_ast::ast::{Argument, BindingPattern, CallExpression, Expression, Function, Statement};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use super::checksum::checksum_step;
use super::entries::Prepared;
use super::{DeadTerms, Session, DEAD_TERM_THREW};
use crate::bail::{Bail, EvalResult};
use crate::frame::{Frame, FrameRef};
use crate::names::Name;
use crate::object;
use crate::ops::to_boolean;
use crate::state::recipe::{
    apply_recipe, compile_recipe, fold_constant, ContainerRef, GateRef, GateTest, KeyRecipe, Prim,
};
use crate::state::{self, RecipeInsert};
use crate::value::{Closure, FnCode, ObjKind, ObjRef, PropKey, Value};

const CONVERSIONS: [&str; 4] = ["parseInt", "parseFloat", "Number", "String"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoRecipeStats {
    pub units: usize,
    pub recipe_terms: usize,
    pub probed_terms: usize,
    pub checked_terms: usize,
    pub disagreements: usize,
}

const NO_STATS: MemoRecipeStats = MemoRecipeStats {
    units: 0,
    recipe_terms: 0,
    probed_terms: 0,
    checked_terms: 0,
    disagreements: 0,
};

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeMode {
    Default,
    Commit,
    ProbeOnly,
}

thread_local! {
    static STATS: Cell<MemoRecipeStats> = const { Cell::new(NO_STATS) };
    static MODE: Cell<RecipeMode> = const { Cell::new(RecipeMode::Default) };
    static OUTCOMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn recipe_mode_tag() -> u8 {
    match MODE.with(Cell::get) {
        RecipeMode::Default => 0,
        RecipeMode::Commit => 1,
        RecipeMode::ProbeOnly => 2,
    }
}

#[doc(hidden)]
pub fn set_recipe_mode_for_tests(mode: RecipeMode) {
    MODE.with(|cell| cell.set(mode));
}

#[doc(hidden)]
pub fn take_rotation_outcomes_for_tests() -> Vec<String> {
    OUTCOMES.with(|outcomes| std::mem::take(&mut *outcomes.borrow_mut()))
}

#[doc(hidden)]
pub fn memo_recipe_stats_for_tests() -> MemoRecipeStats {
    STATS.with(Cell::get)
}

fn count(update: impl FnOnce(&mut MemoRecipeStats)) {
    STATS.with(|cell| {
        let mut stats = cell.get();
        update(&mut stats);
        cell.set(stats);
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    Insert,
    Breach,
    Repeat,
}

struct Planned<'a> {
    container: ObjRef<'a>,
    key: PropKey,
    expect: Expect,
}

type Recipes = rustc_hash::FxHashMap<(usize, usize), Option<Rc<KeyRecipe>>>;

#[derive(Default)]
pub(crate) struct RecipeCache {
    recipes: Recipes,
    counted: bool,
}

impl<'a> Session<'a> {
    pub(super) fn dead_checksum_terms(
        &mut self,
        prepared: &Prepared<'a>,
        from: usize,
        scope: &FrameRef<'a>,
    ) -> EvalResult<()> {
        if self.dead_terms == DeadTerms::Skip || from >= prepared.terms.len() {
            return Ok(());
        }
        let _probe = crate::state::probe_dead_terms();
        for index in from..prepared.terms.len() {
            self.dead_term(prepared, index, scope)?;
        }
        Ok(())
    }

    pub(super) fn note_rotation_outcome(&self, applied: &EvalResult<Option<usize>>) {
        if MODE.with(Cell::get) == RecipeMode::Default {
            return;
        }
        let items: Vec<String> = self
            .elements()
            .iter()
            .map(|item| match item {
                Value::Str(text) => text.to_lossy_string(),
                other => format!("{other:?}"),
            })
            .collect();
        let outcome = format!("{applied:?} {items:?}");
        OUTCOMES.with(|outcomes| outcomes.borrow_mut().push(outcome));
    }

    fn dead_term(
        &mut self,
        prepared: &Prepared<'a>,
        index: usize,
        scope: &FrameRef<'a>,
    ) -> EvalResult<()> {
        let mode = MODE.with(Cell::get);
        let term = prepared.terms.get(index).copied();
        let plan = match term {
            Some(term) if self.dead_tail_proven && mode != RecipeMode::ProbeOnly => {
                self.plan_term(term, scope)
            }
            _ => None,
        };
        #[cfg(feature = "test-hooks")]
        let planned = plan.as_ref().map(|plan| Value::Number(plan.len() as f64));
        #[cfg(feature = "test-hooks")]
        super::note_outcome(
            "dead term",
            planned.as_ref(),
            self.interp.budget.steps_used(),
        );
        let Some(plan) = plan else {
            count(|stats| stats.probed_terms += 1);
            return self.probe_term(prepared, index, scope);
        };
        self.count_unit();
        if cfg!(debug_assertions) && mode == RecipeMode::Default {
            return self.cross_check(prepared, index, scope, &plan);
        }
        count(|stats| stats.recipe_terms += 1);
        for planned in plan
            .iter()
            .filter(|planned| planned.expect != Expect::Repeat)
        {
            state::note_recipe_miss(&planned.container, &planned.key);
        }
        Ok(())
    }

    fn count_unit(&mut self) {
        if !self.recipes.counted {
            self.recipes.counted = true;
            count(|stats| stats.units += 1);
        }
    }

    fn probe_term(
        &mut self,
        prepared: &Prepared<'a>,
        index: usize,
        scope: &FrameRef<'a>,
    ) -> EvalResult<()> {
        let probed = checksum_step(self.eval_term(prepared, index, scope))?;
        #[cfg(feature = "test-hooks")]
        super::note_outcome("probe", probed.as_ref(), self.interp.budget.steps_used());
        if probed.is_none() {
            return Err(Bail::Unsupported(&DEAD_TERM_THREW));
        }
        Ok(())
    }

    fn cross_check(
        &mut self,
        prepared: &Prepared<'a>,
        index: usize,
        scope: &FrameRef<'a>,
        plan: &[Planned<'a>],
    ) -> EvalResult<()> {
        let before = (state::skipped_count(), state::violation_pending());
        let notes = self.interp.notes.len();
        let threw = {
            let checked = checksum_step(self.eval_term(prepared, index, scope))?;
            #[cfg(feature = "test-hooks")]
            super::note_outcome(
                "cross-check",
                checked.as_ref(),
                self.interp.budget.steps_used(),
            );
            checked.is_none()
        };
        let quiet = notes == self.interp.notes.len();
        let agrees = quiet && probe_agrees(plan, before, threw);
        count(|stats| {
            stats.checked_terms += 1;
            stats.disagreements += usize::from(!agrees);
        });
        debug_assert!(
            agrees,
            "the memo-key recipe disagrees with the probe at {:?}",
            prepared.terms.get(index).map(|term| term.span())
        );
        if threw {
            return Err(Bail::Unsupported(&DEAD_TERM_THREW));
        }
        Ok(())
    }
}

fn probe_agrees(plan: &[Planned<'_>], before: (usize, bool), threw: bool) -> bool {
    let violated = state::violation_pending();
    if plan.iter().any(|planned| planned.expect == Expect::Breach) {
        return violated;
    }
    let inserts: Vec<&Planned<'_>> = plan
        .iter()
        .filter(|planned| planned.expect == Expect::Insert)
        .collect();
    let inserted = inserts
        .iter()
        .all(|planned| state::skipped_here(&planned.container, &planned.key));
    !threw && violated == before.1 && state::skipped_count() == before.0 + inserts.len() && inserted
}

impl<'a> Session<'a> {
    fn plan_term(
        &mut self,
        term: &'a Expression<'a>,
        scope: &FrameRef<'a>,
    ) -> Option<Vec<Planned<'a>>> {
        if !state::invocation_quiet() {
            return None;
        }
        let mut calls = Vec::new();
        if !self.term_calls(term, scope, &mut calls) {
            return None;
        }
        let mut plan: Vec<Planned<'a>> = Vec::with_capacity(calls.len());
        for call in calls {
            let (container, key) = self.recipe_key_of(call, scope)?;
            let expect = self.expect_of(&plan, &container, &key)?;
            plan.push(Planned {
                container,
                key,
                expect,
            });
        }
        Some(plan)
    }

    fn term_calls(
        &mut self,
        expr: &'a Expression<'a>,
        scope: &FrameRef<'a>,
        out: &mut Vec<&'a CallExpression<'a>>,
    ) -> bool {
        match expr {
            Expression::NumericLiteral(_) | Expression::StringLiteral(_) => true,
            Expression::ParenthesizedExpression(node) => {
                self.term_calls(&node.expression, scope, out)
            }
            Expression::UnaryExpression(node) => {
                let numeric = matches!(
                    node.operator,
                    UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
                );
                numeric && self.term_calls(&node.argument, scope, out)
            }
            Expression::BinaryExpression(node) => {
                let arithmetic = matches!(
                    node.operator,
                    BinaryOperator::Addition
                        | BinaryOperator::Subtraction
                        | BinaryOperator::Multiplication
                        | BinaryOperator::Division
                        | BinaryOperator::Remainder
                );
                arithmetic
                    && self.term_calls(&node.left, scope, out)
                    && self.term_calls(&node.right, scope, out)
            }
            Expression::CallExpression(call) => self.call_in_term(call, scope, out, false),
            _ => false,
        }
    }

    fn call_in_term(
        &mut self,
        call: &'a CallExpression<'a>,
        scope: &FrameRef<'a>,
        out: &mut Vec<&'a CallExpression<'a>>,
        converted: bool,
    ) -> bool {
        let Expression::Identifier(callee) = &call.callee else {
            return false;
        };
        if call.optional {
            return false;
        }
        let name = callee.name.as_str();
        if !CONVERSIONS.contains(&name) || !self.is_builtin(name, scope) {
            out.push(call);
            return true;
        }
        match call.arguments.as_slice() {
            [Argument::CallExpression(inner)] if !converted => {
                self.call_in_term(inner, scope, out, true)
            }
            _ => false,
        }
    }

    fn is_builtin(&mut self, name: &'a str, scope: &FrameRef<'a>) -> bool {
        let name = self.interp.name_of(name);
        let root = self.interp.global_env();
        match (
            Frame::slot_by_name(scope, name),
            Frame::slot_by_name(&root, name),
        ) {
            (Ok(Some(Value::Object(found))), Ok(Some(Value::Object(global)))) => {
                Rc::ptr_eq(&found, &global) && matches!(found.borrow().kind, ObjKind::Native(_, _))
            }
            _ => false,
        }
    }
}

impl<'a> Session<'a> {
    fn recipe_key_of(
        &mut self,
        call: &'a CallExpression<'a>,
        scope: &FrameRef<'a>,
    ) -> Option<(ObjRef<'a>, PropKey)> {
        let (callee, args) = self.resolve_decoder(call, scope)?;
        let decoder = state::machinery_owner(&callee)?;
        let closure = closure_of(&callee)?;
        let FnCode::Function(function) = closure.code else {
            return None;
        };
        let recipe = self.recipe_for(function, decoder)?;
        self.recipe_target(&recipe, &callee, &closure.env, &args)
    }

    fn resolve_decoder(
        &mut self,
        call: &'a CallExpression<'a>,
        scope: &FrameRef<'a>,
    ) -> Option<(ObjRef<'a>, Vec<Value<'a>>)> {
        let Expression::Identifier(id) = &call.callee else {
            return None;
        };
        let args = constant_arguments(&call.arguments, &[])?;
        let name = self.interp.name_of(id.name.as_str());
        let target = Frame::slot_by_name(scope, name).ok()??.as_object()?.clone();
        if state::machinery_owner(&target).is_some() {
            return Some((target, args));
        }
        self.through_wrapper(&target, &args)
    }

    fn through_wrapper(
        &mut self,
        wrapper: &ObjRef<'a>,
        args: &[Value<'a>],
    ) -> Option<(ObjRef<'a>, Vec<Value<'a>>)> {
        let closure = closure_of(wrapper)?;
        let FnCode::Function(function) = closure.code else {
            return None;
        };
        let (callee, inner) = forwarded_call(function)?;
        let bindings = parameter_bindings(function, args)?;
        let inner_args = constant_arguments(inner, &bindings)?;
        let name = self.interp.name_of(callee);
        let target = Frame::slot_by_name(&closure.env, name)
            .ok()??
            .as_object()?
            .clone();
        Some((target, inner_args))
    }

    fn recipe_for(&mut self, function: &'a Function<'a>, decoder: usize) -> Option<Rc<KeyRecipe>> {
        let key = (std::ptr::from_ref(function) as usize, decoder);
        if let Some(found) = self.recipes.recipes.get(&key) {
            return found.clone();
        }
        let self_name = self.decoders.get(decoder)?.source_name;
        let interp = &mut self.interp;
        let compiled = compile_recipe(function, self_name, &mut |text| interp.name_of(text));
        let compiled = compiled.map(Rc::new);
        self.recipes.recipes.insert(key, compiled.clone());
        compiled
    }

    fn recipe_target(
        &self,
        recipe: &KeyRecipe,
        callee: &ObjRef<'a>,
        env: &FrameRef<'a>,
        args: &[Value<'a>],
    ) -> Option<(ObjRef<'a>, PropKey)> {
        if recipe.reads_self() && !Rc::ptr_eq(&bound_object(env, recipe.self_name)?, callee) {
            return None;
        }
        if let Some(gate) = &recipe.gate {
            self.gate_passed(gate, callee, env, recipe.undefined)?;
        }
        let session_array = self.array.as_object()?;
        if let Some(name) = recipe.array {
            Rc::ptr_eq(&bound_object(env, name)?, session_array).then_some(())?;
        }
        let key = apply_recipe(recipe, args, session_array)?;
        let container = match &recipe.container {
            ContainerRef::Binding(name) => bound_object(env, *name)?,
            ContainerRef::SelfMember(segment) => {
                object::own_property(callee, segment)?.as_object()?.clone()
            }
        };
        Some((container, key))
    }

    fn gate_passed(
        &self,
        gate: &GateRef,
        callee: &ObjRef<'a>,
        env: &FrameRef<'a>,
        undefined: Name,
    ) -> Option<()> {
        let flag = object::own_property(callee, &gate.flag)?;
        let skipped = match gate.test {
            GateTest::Undefined { loose } => {
                let unshadowed = matches!(
                    Frame::slot_by_name(env, undefined),
                    Ok(Some(Value::Undefined))
                );
                let nullish = matches!(flag, Value::Null) && loose;
                unshadowed && !matches!(flag, Value::Undefined) && !nullish
            }
            GateTest::Falsy => to_boolean(&flag),
        };
        skipped.then_some(())
    }

    fn expect_of(
        &self,
        plan: &[Planned<'a>],
        container: &ObjRef<'a>,
        key: &PropKey,
    ) -> Option<Expect> {
        if self.interp.realm.hostile_name(container).is_some() {
            return None;
        }
        let repeated = plan
            .iter()
            .any(|planned| Rc::ptr_eq(&planned.container, container) && planned.key == *key);
        if repeated {
            return Some(Expect::Repeat);
        }
        if object::has_own_property(container, key) {
            return state::skipped_here(container, key).then_some(Expect::Repeat);
        }
        if inherits(container, key) {
            return None;
        }
        if !self.miss_reaches_probe(container, key) {
            return None;
        }
        match state::predict_recipe_miss(container, key) {
            RecipeInsert::Inserted => Some(Expect::Insert),
            RecipeInsert::Breach => Some(Expect::Breach),
            RecipeInsert::Hit | RecipeInsert::Unmarked => None,
        }
    }

    fn miss_reaches_probe(&self, container: &ObjRef<'a>, key: &PropKey) -> bool {
        let target = Value::Object(container.clone());
        let Ok(inherited) = self.interp.realm.intrinsic_member(&target, key) else {
            return false;
        };
        let arguments = object::is_arguments(container);
        if arguments && object::arguments_miss_differs(key, inherited.as_ref()) {
            return false;
        }
        let unmodelled = object::decoding() && crate::unmodelled::miss_is_unmodelled(&target, key);
        inherited.is_none() && !object::is_constructor_key(key) && !unmodelled
    }
}

fn closure_of<'a>(function: &ObjRef<'a>) -> Option<Rc<Closure<'a>>> {
    match &function.borrow().kind {
        ObjKind::Function(closure) => Some(Rc::clone(closure)),
        _ => None,
    }
}

fn bound_object<'a>(env: &FrameRef<'a>, name: Name) -> Option<ObjRef<'a>> {
    Frame::slot_by_name(env, name).ok()??.as_object().cloned()
}

fn constant_arguments<'a>(
    arguments: &[Argument<'_>],
    bindings: &[(&str, Option<Prim>)],
) -> Option<Vec<Value<'a>>> {
    arguments
        .iter()
        .map(|argument| match argument {
            Argument::SpreadElement(_) => None,
            other => fold_constant(other.to_expression(), bindings).map(|prim| prim.value()),
        })
        .collect()
}

fn parameter_bindings<'f>(
    function: &'f Function<'_>,
    args: &[Value<'_>],
) -> Option<Vec<(&'f str, Option<Prim>)>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .enumerate()
        .map(
            |(index, param)| match (&param.pattern, &param.initializer) {
                (BindingPattern::BindingIdentifier(id), None) => {
                    Some((id.name.as_str(), args.get(index).and_then(Prim::of)))
                }
                _ => None,
            },
        )
        .collect()
}

fn forwarded_call<'a>(function: &'a Function<'a>) -> Option<(&'a str, &'a [Argument<'a>])> {
    if function.generator || function.r#async {
        return None;
    }
    let [Statement::ReturnStatement(ret)] = function.body.as_ref()?.statements.as_slice() else {
        return None;
    };
    let Expression::CallExpression(call) = ret.argument.as_ref()? else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    let name = callee.name.as_str();
    let shadowed = name == "arguments"
        || function.id.as_ref().is_some_and(|id| id.name == name)
        || function.params.items.iter().any(|param| {
            matches!(&param.pattern, BindingPattern::BindingIdentifier(id) if id.name == name)
        });
    (!call.optional && !shadowed).then_some((name, call.arguments.as_slice()))
}

fn inherits(container: &ObjRef<'_>, key: &PropKey) -> bool {
    let Some(start) = container.borrow().proto.clone() else {
        return false;
    };
    let found = object::walk_prototype_chain(start, |link| {
        object::has_own_property(link, key).then_some(())
    });
    !matches!(found, Ok(None))
}
