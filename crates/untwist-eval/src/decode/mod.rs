mod checksum;
mod dead_terms;
mod entries;
mod kept;

use std::rc::Rc;

use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, Function};
use oxc_ast::builder::GetAstBuilder;
use oxc_span::Span;
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashSet;

use untwist_core::ast_util::literal::undefined_expression;

use crate::bail::{Abrupt, AbruptKind, Bail, EvalResult};
use crate::budget::Budget;
use crate::env::declare;
use crate::frame::{Frame, FrameRef};
use crate::heap::HeapLedger;
use crate::interp::Interp;
use crate::names::Name;
use crate::object::{
    self, array_element, array_items, array_length, array_rotate_left, with_array_items, ProtoRead,
};
use crate::ops::{binary_op, strict_equals};
use crate::rotation::{machinery_key, RotationKey, RotationMemo};
use crate::value::{js_str_in, FnCode, JsString, Value};

pub use checksum::{checksum_from_rotator, Checksum};
use checksum::{checksum_step, rotator_body_is_strict, NATIVE_ROTATE_STEP_COST};
pub(crate) use dead_terms::recipe_mode_tag;
pub use dead_terms::{
    memo_recipe_stats_for_tests, set_recipe_mode_for_tests, take_rotation_outcomes_for_tests,
    MemoRecipeStats, RecipeMode,
};
use entries::Prepared;
pub(crate) use kept::KeptUnits;
#[cfg(feature = "test-hooks")]
pub(crate) use kept::{set_arguments_unkeyed, set_keep_from_steps, set_keeping};

pub const DECODE_ERROR_MARKER: &str = "untwist:decode_error";
pub const DEAD_TERM_THREW: &str =
    "a skipped checksum term threw while untwist probed its memo keys";
pub const ROTATOR_THREW: &str =
    "the string array's rotator throws, so the program stops there and no later call site is proven";

pub struct FnRef<'a> {
    pub name: &'a str,
    pub function: &'a Function<'a>,
    pub strict: bool,
}

pub struct DecoderProgram<'a> {
    pub source_text: &'a str,
    pub string_array: FnRef<'a>,
    pub decoders: Vec<FnRef<'a>>,
    pub rotator: Option<&'a Expression<'a>>,
    pub strict: bool,
}

impl<'a> DecoderProgram<'a> {
    pub fn functions(&self) -> Vec<&'a Function<'a>> {
        std::iter::once(self.string_array.function)
            .chain(self.decoders.iter().map(|decoder| decoder.function))
            .collect()
    }

    pub fn expressions(&self) -> Vec<&'a Expression<'a>> {
        self.rotator.into_iter().collect()
    }
}

pub struct Call<'a> {
    pub decoder: usize,
    pub span: Span,
    pub arguments: Vec<&'a Expression<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decoded {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(JsString),
    Object,
}

impl Decoded {
    fn of(value: Value<'_>) -> Self {
        match value {
            Value::Str(text) => Self::Str(text),
            Value::Number(n) => Self::Num(n),
            Value::Bool(flag) => Self::Bool(flag),
            Value::Null => Self::Null,
            Value::Undefined => Self::Undefined,
            Value::Object(_) => Self::Object,
        }
    }
}

pub trait Evaluator<'a> {
    fn decode(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
    ) -> EvalResult<Vec<Decoded>>;
}

pub struct InterpEvaluator {
    pub budget: Budget,
    pub heap: Option<Rc<HeapLedger>>,
    pub notes: Vec<String>,
    pub proto_reads: Vec<ProtoRead>,
    pub builtin_roots: Vec<&'static str>,
}

impl InterpEvaluator {
    pub fn new(budget: Budget) -> Self {
        Self {
            budget,
            heap: None,
            notes: Vec::new(),
            proto_reads: Vec::new(),
            builtin_roots: Vec::new(),
        }
    }
}

impl Default for InterpEvaluator {
    fn default() -> Self {
        Self::new(Budget::default())
    }
}

pub(crate) struct DecoderSlot<'a> {
    name: Name,
    source_name: &'a str,
    current: Value<'a>,
    _registered_self: Value<'a>,
    settled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeadTerms {
    Skip,
    Probe { miss_paths_proven: bool },
}

pub struct Session<'a> {
    pub(crate) interp: Interp<'a>,
    pub(crate) dead_terms: DeadTerms,
    pub(crate) dead_tail_proven: bool,
    pub(crate) env: FrameRef<'a>,
    pub(crate) array: Value<'a>,
    pub(crate) decoders: Vec<DecoderSlot<'a>>,
    pub(crate) notes: Vec<String>,
    pub(crate) instantiation_proto_reads: Vec<ProtoRead>,
    pub(crate) instantiation_builtin_roots: Vec<&'static str>,
    pub(crate) recipes: dead_terms::RecipeCache,
    pub(crate) entries: entries::Entries<'a>,
}

impl<'realm> Session<'realm> {
    pub fn root_env(&self) -> &FrameRef<'realm> {
        &self.env
    }

    pub fn array_len(&self) -> usize {
        self.array.as_object().and_then(array_length).unwrap_or(0)
    }

    pub fn element(&self, index: usize) -> Option<Value<'realm>> {
        array_element(self.array.as_object()?, index)
    }

    pub fn array_first(&self) -> Option<Value<'realm>> {
        self.element(0)
    }

    pub fn elements(&self) -> Vec<Value<'realm>> {
        self.array
            .as_object()
            .and_then(array_items)
            .unwrap_or_default()
    }

    pub fn decoder(&self, index: usize) -> Option<Value<'realm>> {
        self.decoders.get(index).map(|slot| slot.current.clone())
    }

    pub fn eval_call(&mut self, call: &Call<'realm>) -> EvalResult<Value<'realm>> {
        let callee = self.current_decoder(call.decoder)?;
        let args = self.eval_call_arguments(call)?;
        let value = self.dispatch_decoder_call(&callee, &args)?;
        self.settle_decoder(call.decoder)?;
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_site(self.interp.budget.steps_used());
        Ok(value)
    }

    pub fn notes(&self) -> Vec<String> {
        let mut all = self.notes.clone();
        all.extend(self.interp.notes.iter().cloned());
        all
    }

    pub fn drain_notes(&mut self) -> Vec<String> {
        let mut all = std::mem::take(&mut self.notes);
        all.append(&mut self.interp.notes);
        all
    }

    pub fn drain_instantiation_proto_reads(&mut self) -> Vec<ProtoRead> {
        std::mem::take(&mut self.instantiation_proto_reads)
    }

    pub fn drain_instantiation_builtin_roots(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.instantiation_builtin_roots)
    }

    pub fn budget(&self) -> Budget {
        self.interp.budget
    }
}

#[cfg(feature = "test-hooks")]
pub(crate) fn note_outcome(what: &str, value: Option<&Value<'_>>, steps: u64) {
    crate::test_hooks::note_attempt(|| {
        let rendered = value.map_or_else(|| "none".to_string(), crate::test_hooks::render_value);
        format!("{what}: {rendered} at {steps}")
    });
}

pub(crate) fn to_bail(abrupt: Abrupt<'_>) -> Bail {
    match abrupt.into_kind() {
        AbruptKind::Bail(bail) => bail,
        AbruptKind::Throw(_) => {
            Bail::Unsupported(&"uncaught throw while instantiating the decoder program")
        }
    }
}

impl InterpEvaluator {
    pub fn instantiate<'a>(&mut self, program: &DecoderProgram<'a>) -> EvalResult<Session<'a>> {
        crate::interp::lexical::refuse_dead_zones(&program.functions(), &program.expressions())?;
        let mut interp = Interp::new_for_decode(program.source_text, self.budget);
        interp.set_heap(self.heap.clone());
        interp.strict = program.strict;
        let env = interp.global_env();
        object::set_tracing(true);

        interp.strict = program.string_array.strict;
        let array_fn = interp.make_closure(FnCode::Function(program.string_array.function), &env);
        let array_name = interp.name_of(program.string_array.name);
        declare(&env, array_name, array_fn.clone());
        let decoders = declare_decoders(&mut interp, &program.decoders, &env);
        interp.strict = program.strict;

        let array = interp.call(&array_fn, Value::Undefined, &[]);
        let proto_reads = object::take_trace();
        let builtin_roots = object::take_builtin_roots();
        object::set_tracing(false);
        self.budget = interp.budget;
        let array = array.map_err(to_bail)?;
        let Some(length) = array.as_object().and_then(array_length) else {
            return Err(Bail::Unsupported(
                &"the string-array function did not return an array",
            ));
        };
        interp.loop_len_hint = length;

        self.budget = interp.budget;
        Ok(Session {
            interp,
            dead_terms: DeadTerms::Skip,
            dead_tail_proven: false,
            env,
            array,
            decoders,
            notes: Vec::new(),
            instantiation_proto_reads: proto_reads,
            instantiation_builtin_roots: builtin_roots,
            recipes: dead_terms::RecipeCache::default(),
            entries: entries::Entries::default(),
        })
    }
}

fn declare_decoders<'a>(
    interp: &mut Interp<'a>,
    functions: &[FnRef<'a>],
    env: &FrameRef<'a>,
) -> Vec<DecoderSlot<'a>> {
    let mut decoders = Vec::with_capacity(functions.len());
    for (index, decoder) in functions.iter().enumerate() {
        interp.strict = decoder.strict;
        let closure = interp.make_closure(FnCode::Function(decoder.function), env);
        let decoder_name = interp.name_of(decoder.name);
        declare(env, decoder_name, closure.clone());
        if let Some(obj) = closure.as_object() {
            crate::state::register_machinery_self(obj, index);
        }
        decoders.push(DecoderSlot {
            name: decoder_name,
            source_name: decoder.name,
            current: closure.clone(),
            _registered_self: closure,
            settled: false,
        });
    }
    decoders
}

impl<'a> Session<'a> {
    pub fn rotate(&mut self, steps: usize) -> EvalResult<()> {
        let Some(obj) = self.array.as_object().cloned() else {
            return Err(Bail::Unsupported(&"string array is not an array"));
        };
        for _ in 0..steps {
            crate::state::note_rotation_step();
        }
        match array_rotate_left(&obj, steps) {
            Some(()) => Ok(()),
            None => Err(Bail::Unsupported(&"string array is not an array")),
        }
    }

    fn find_rotation(&mut self, prepared: &Prepared<'a>) -> EvalResult<Option<usize>> {
        let length = self.array_len();
        let Some((scope, expected)) = self.checksum_scope(prepared)? else {
            return Ok(None);
        };
        for attempt in 0..length {
            if self.checksum_attempt(prepared, &scope, &expected)? {
                return Ok(Some(attempt));
            }
            self.interp
                .budget
                .spend(1 + (length as u64) / NATIVE_ROTATE_STEP_COST)?;
            self.rotate(1)?;
        }
        Ok(None)
    }

    fn checksum_attempt(
        &mut self,
        prepared: &Prepared<'a>,
        scope: &FrameRef<'a>,
        expected: &Value<'a>,
    ) -> EvalResult<bool> {
        let produced = self.checksum_sum(prepared, scope, expected)?;
        #[cfg(feature = "test-hooks")]
        note_outcome(
            "attempt",
            produced.as_ref(),
            self.interp.budget.steps_used(),
        );
        let Some(produced) = produced else {
            return Ok(false);
        };
        Ok(strict_equals(&produced, expected))
    }

    fn checksum_sum(
        &mut self,
        prepared: &Prepared<'a>,
        scope: &FrameRef<'a>,
        expected: &Value<'a>,
    ) -> EvalResult<Option<Value<'a>>> {
        let expected_is_number = matches!(expected, Value::Number(_));
        let mut total: Option<Value<'a>> = None;
        for index in 0..prepared.terms.len() {
            let value = checksum_step(self.eval_term(prepared, index, scope))?;
            #[cfg(feature = "test-hooks")]
            note_outcome("term", value.as_ref(), self.interp.budget.steps_used());
            let Some(value) = value else {
                return Ok(None);
            };
            total = match total {
                None => Some(value),
                Some(previous) => {
                    let sum = binary_op(
                        BinaryOperator::Addition,
                        &previous,
                        &value,
                        &mut self.interp,
                    );
                    match checksum_step(sum)? {
                        Some(folded) => Some(folded),
                        None => return Ok(None),
                    }
                }
            };
            let nan = matches!(total, Some(Value::Number(n)) if n.is_nan());
            if expected_is_number && nan && self.skips_dead_terms() {
                self.dead_checksum_terms(prepared, index + 1, scope)?;
                return Ok(None);
            }
        }
        Ok(total)
    }

    fn skips_dead_terms(&self) -> bool {
        self.dead_terms == DeadTerms::Skip || self.dead_tail_proven
    }

    fn prove_dead_tail(&self, program: &DecoderProgram<'a>, checksum: &Checksum<'a>) -> bool {
        let DeadTerms::Probe { miss_paths_proven } = self.dead_terms else {
            return false;
        };
        let machinery: Vec<&str> = std::iter::once(program.string_array.name)
            .chain(program.decoders.iter().map(|decoder| decoder.name))
            .collect();
        miss_paths_proven && checksum.dead_tail_provable(&machinery)
    }

    fn rotation_key(&self, program: &DecoderProgram<'a>) -> Option<RotationKey> {
        let object = self.array.as_object()?;
        with_array_items(object, |items| machinery_key(program, items)).flatten()
    }

    fn confirm_rotation(&mut self, prepared: &Prepared<'a>, rotation: usize) -> EvalResult<bool> {
        let length = self.array_len();
        if rotation >= length {
            return Ok(false);
        }
        let Some((scope, expected)) = self.checksum_scope(prepared)? else {
            return Ok(false);
        };
        self.interp
            .budget
            .spend(1 + (length as u64) / NATIVE_ROTATE_STEP_COST)?;
        self.rotate(rotation)?;
        if self.checksum_attempt(prepared, &scope, &expected)? {
            return Ok(true);
        }
        self.rotate(length - rotation)?;
        Ok(false)
    }

    fn memoised_rotation(
        &mut self,
        program: &DecoderProgram<'a>,
        prepared: &Prepared<'a>,
        memo: &mut RotationMemo,
    ) -> EvalResult<Option<usize>> {
        let key = match self.dead_terms {
            DeadTerms::Skip => self.rotation_key(program),
            DeadTerms::Probe { .. } => None,
        };
        if let Some(rotation) = key.as_ref().and_then(|key| memo.cached(key)) {
            if self.confirm_rotation(prepared, rotation)? {
                memo.record_hit();
                return Ok(Some(rotation));
            }
        }
        memo.record_search();
        let found = self.find_rotation(prepared)?;
        if let (Some(key), Some(rotation)) = (key, found) {
            memo.store(key, rotation);
        }
        Ok(found)
    }

    pub fn apply_rotation(&mut self, program: &DecoderProgram<'a>) -> EvalResult<Option<usize>> {
        self.apply_rotation_with(program, &mut RotationMemo::default())
    }

    pub fn apply_rotation_with(
        &mut self,
        program: &DecoderProgram<'a>,
        memo: &mut RotationMemo,
    ) -> EvalResult<Option<usize>> {
        let Some(iife) = program.rotator else {
            return Ok(None);
        };
        object::set_decoding(true);
        let applied = self.rotate_by(iife, program, memo);
        self.note_rotation_outcome(&applied);
        object::set_decoding(false);
        applied
    }

    fn rotate_by(
        &mut self,
        iife: &'a Expression<'a>,
        program: &DecoderProgram<'a>,
        memo: &mut RotationMemo,
    ) -> EvalResult<Option<usize>> {
        match checksum_from_rotator(iife) {
            Some(checksum) => {
                self.dead_tail_proven = self.prove_dead_tail(program, &checksum);
                let prepared = self.prepare(checksum);
                let context = self.interp.strict;
                self.interp.strict = context || rotator_body_is_strict(iife);
                let found = self.memoised_rotation(program, &prepared, memo);
                self.interp.strict = context;
                if let Some(found) = found? {
                    return Ok(Some(found));
                }
                self.notes.push(
                    "checksum brute force found no rotation; interpreting the rotator".to_string(),
                );
            }
            None => {
                self.notes.push(
                    "no checksum could be extracted from the rotator; interpreting it directly"
                        .to_string(),
                );
            }
        }
        self.run_rotator(iife)?;
        Ok(None)
    }
}

impl<'a> Session<'a> {
    fn dispatch_decoder_call(
        &mut self,
        callee: &Value<'a>,
        args: &[Value<'a>],
    ) -> EvalResult<Value<'a>> {
        match self.interp.call(callee, Value::Undefined, args) {
            Ok(value) => Ok(value),
            Err(abrupt) => match abrupt.into_kind() {
                AbruptKind::Bail(bail) => Err(bail),
                AbruptKind::Throw(_) => Err(Bail::Unsupported(&"a decoder call threw")),
            },
        }
    }

    fn current_decoder(&self, index: usize) -> EvalResult<Value<'a>> {
        let Some(slot) = self.decoders.get(index) else {
            return Err(Bail::Unsupported(&"call site names an unknown decoder"));
        };
        if slot.settled {
            return Ok(slot.current.clone());
        }
        match Frame::slot_by_name(&self.env, slot.name).map_err(Bail::from)? {
            Some(current) => Ok(current),
            None => Err(Bail::unresolved_global(slot.source_name)),
        }
    }

    fn settle_decoder(&mut self, index: usize) -> EvalResult<()> {
        let Some(slot) = self.decoders.get(index) else {
            return Ok(());
        };
        if slot.settled {
            return Ok(());
        }
        let name = slot.name;
        let source_name = slot.source_name;
        let Some(current) = Frame::slot_by_name(&self.env, name).map_err(Bail::from)? else {
            return Err(Bail::unresolved_global(source_name));
        };
        let slot = &mut self.decoders[index];
        slot.settled = true;
        slot.current = current;
        Ok(())
    }
}

#[inline(never)]
fn candidate_shape(
    program: &DecoderProgram<'_>,
    escaped_decoders: &FxHashSet<usize>,
) -> EvalResult<Option<crate::state::Shape>> {
    match crate::state::shape(program) {
        crate::state::MachineryState::Refused(reason) => Err(Bail::Unsupported(reason)),
        crate::state::MachineryState::Candidate(shape) => {
            let escapes =
                (0..program.decoders.len()).any(|index| escaped_decoders.contains(&index));
            if escapes {
                return Err(Bail::Unsupported(
                    &"a stateful decoder is referenced outside the calls untwist proves",
                ));
            }
            Ok(Some(shape))
        }
        crate::state::MachineryState::Stateless => Ok(None),
    }
}

impl InterpEvaluator {
    pub fn decode_with_memo<'a>(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
        memo: &mut RotationMemo,
        escaped_decoders: &FxHashSet<usize>,
    ) -> EvalResult<Vec<Decoded>> {
        let values = self.decode_partial_with_memo(program, calls, memo, escaped_decoders)?;
        values
            .into_iter()
            .map(|value| value.ok_or(Bail::Budget))
            .collect()
    }

    pub fn decode_partial_with_memo<'a>(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
        memo: &mut RotationMemo,
        escaped_decoders: &FxHashSet<usize>,
    ) -> EvalResult<Vec<Option<Decoded>>> {
        let start = kept::Start::of(self);
        let unit = (program, calls, escaped_decoders);
        let early = memo.units.early_key(&start, unit);
        if let Some(kept) = early.and_then(|key| memo.units.replay(&key, self)) {
            return kept;
        }
        #[cfg(feature = "test-hooks")]
        let mark = crate::test_hooks::UnitMark::decode(self, memo);
        let result = self.decode_unit(program, calls, memo, escaped_decoders);
        #[cfg(feature = "test-hooks")]
        mark.finish(self, memo, &result);
        let result = result.map(|values| {
            values
                .into_iter()
                .map(|value| value.map(Decoded::of))
                .collect()
        });
        memo.units.keep((early, start), unit, self, &result);
        result
    }

    fn decode_unit<'a>(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
        memo: &mut RotationMemo,
        escaped_decoders: &FxHashSet<usize>,
    ) -> EvalResult<Vec<Option<Value<'a>>>> {
        crate::heap::spent_run_budget()?;
        let shape = candidate_shape(program, escaped_decoders)?;
        let result = match &shape {
            Some(shape) => self.decode_with_memo_monitored(program, calls, memo, shape),
            None => self.decode_with_memo_traced(program, calls, memo, DeadTerms::Skip),
        };
        self.proto_reads.extend(object::take_trace());
        self.builtin_roots.extend(object::take_builtin_roots());
        object::set_tracing(false);
        result
    }

    fn decode_with_memo_monitored<'a>(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
        memo: &mut RotationMemo,
        shape: &crate::state::Shape,
    ) -> EvalResult<Vec<Option<Value<'a>>>> {
        let _guard = crate::state::set_state_tracing(Some(shape));
        let dead_terms = if shape.memos.is_empty() && !shape.captured_memo {
            DeadTerms::Skip
        } else {
            DeadTerms::Probe {
                miss_paths_proven: shape.miss_paths_proven,
            }
        };
        let result = self.decode_with_memo_traced(program, calls, memo, dead_terms);
        match (result, crate::state::take_violation()) {
            (Ok(values), None) => Ok(values),
            (_, Some(reason)) => Err(Bail::Unsupported(reason)),
            (Err(bail), None) => Err(bail),
        }
    }

    fn decode_with_memo_traced<'a>(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
        memo: &mut RotationMemo,
        dead_terms: DeadTerms,
    ) -> EvalResult<Vec<Option<Value<'a>>>> {
        let mut session = self.instantiate(program)?;
        session.dead_terms = dead_terms;
        let rotated = session.apply_rotation_with(program, memo);
        self.proto_reads
            .extend(session.drain_instantiation_proto_reads());
        self.builtin_roots
            .extend(session.drain_instantiation_builtin_roots());
        if let Err(bail) = rotated {
            self.budget = session.interp.budget;
            return Err(bail);
        }
        object::set_tracing(true);
        let values = eval_calls_until_spent(&mut session, calls);
        self.budget = session.interp.budget;
        let values = values?;
        self.notes.extend(session.drain_notes());
        Ok(values)
    }
}

fn eval_calls_until_spent<'a>(
    session: &mut Session<'a>,
    calls: &[Call<'a>],
) -> EvalResult<Vec<Option<Value<'a>>>> {
    let mut values = Vec::with_capacity(calls.len());
    for call in calls {
        crate::heap::spent_run_budget()?;
        match session.eval_call(call) {
            Ok(value) => values.push(Some(value)),
            Err(Bail::Budget) => break,
            Err(bail) => return Err(bail),
        }
    }
    values.resize(calls.len(), None);
    Ok(values)
}

impl<'a> Evaluator<'a> for InterpEvaluator {
    fn decode(
        &mut self,
        program: &DecoderProgram<'a>,
        calls: &[Call<'a>],
    ) -> EvalResult<Vec<Decoded>> {
        self.decode_with_memo(
            program,
            calls,
            &mut RotationMemo::default(),
            &FxHashSet::default(),
        )
    }
}

pub fn is_decoded_string(value: &Decoded) -> bool {
    matches!(value, Decoded::Str(_))
}

pub fn all_decoded(values: &[Decoded]) -> bool {
    values.iter().all(is_decoded_string)
}

pub fn decoded_to_expression<'a>(
    value: &Decoded,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    match value {
        Decoded::Str(text) => Some(string_literal(text, span, b)),
        Decoded::Bool(flag) => Some(Expression::new_boolean_literal(span, *flag, b)),
        Decoded::Null => Some(Expression::new_null_literal(span, b)),
        Decoded::Undefined => Some(undefined_expression(span, b)),
        Decoded::Num(n) => Some(number_expression(*n, span, b)),
        Decoded::Object => None,
    }
}

fn quotient<'a>(numerator: f64, span: Span, b: &impl GetAstBuilder<'a>) -> Expression<'a> {
    Expression::new_binary_expression(
        span,
        signed_number(numerator, span, b),
        BinaryOperator::Division,
        Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, b),
        b,
    )
}

fn signed_number<'a>(value: f64, span: Span, b: &impl GetAstBuilder<'a>) -> Expression<'a> {
    let literal = Expression::new_numeric_literal(span, value.abs(), None, NumberBase::Decimal, b);
    if value.is_sign_negative() {
        return Expression::new_unary_expression(span, UnaryOperator::UnaryNegation, literal, b);
    }
    literal
}

fn number_expression<'a>(value: f64, span: Span, b: &impl GetAstBuilder<'a>) -> Expression<'a> {
    if value.is_nan() {
        return quotient(0.0, span, b);
    }
    if value.is_infinite() {
        return quotient(if value.is_sign_negative() { -1.0 } else { 1.0 }, span, b);
    }
    if value == 0.0 && value.is_sign_negative() {
        return signed_number(-0.0, span, b);
    }
    Expression::new_numeric_literal(span, value, None, NumberBase::Decimal, b)
}

pub(crate) fn string_literal<'a>(
    text: &JsString,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Expression<'a> {
    let literal = js_str_in(text.units(), b.builder().allocator());
    Expression::new_string_literal(span, literal, None, b)
}
