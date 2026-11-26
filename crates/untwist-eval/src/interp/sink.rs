use std::rc::Rc;

use oxc_ast::ast::{Expression, Function, Program, Statement};
use oxc_span::GetSpan;
use rustc_hash::FxHashSet;

use crate::bail::Bail;
use crate::budget::Budget;
use crate::env::read_binding;
use crate::heap::HeapLedger;
use crate::interp::Interp;
use crate::object::lookup_property;
use crate::value::{Closure, Eval, FnCode, JsString, NativeId, ObjKind, ObjRef, PropKey, Value};

const FUNCTION_HEAD: &str = "function anonymous(";

const READER_MEMBERS: [&str; 12] = [
    "map", "filter", "forEach", "reduce", "some", "every", "find", "toString", "valueOf", "call",
    "apply", "bind",
];

const CONSTRUCTOR_UNITS: &[u16] = &[
    b'c' as u16,
    b'o' as u16,
    b'n' as u16,
    b's' as u16,
    b't' as u16,
    b'r' as u16,
    b'u' as u16,
    b'c' as u16,
    b't' as u16,
    b'o' as u16,
    b'r' as u16,
];

const LINKED_CONSTRUCTORS: [&str; 3] = ["Object", "Number", "RegExp"];

const OUTSIDE_POOL: &str = "a Function text outside the sink pool";

const CONFINED_NATIVE: &str = "a native that may write, called in the sink region";

const CONFINED_CLOSURE: &str = "a function from outside the sink region, called in it";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Watch {
    Off,
    Note,
    Confine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinkCallee {
    Function,
    Eval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkOutcome {
    Reached {
        callee: SinkCallee,
        texts: Vec<String>,
    },
    Unknown(Vec<String>),
    Refused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinkRun {
    pub outcome: SinkOutcome,
    pub prefix_clean: bool,
    pub steps: u64,
    pub heap: u64,
}

pub struct SinkScript<'p, 'a> {
    pub program: &'p Program<'a>,
    pub sink: usize,
    pub region: u32,
    pub pool: Vec<(String, &'p Function<'a>)>,
}

pub(crate) struct SinkState<'a> {
    function: Option<ObjRef<'a>>,
    boolean: Option<ObjRef<'a>>,
    eval: Option<ObjRef<'a>>,
    pool: Vec<(String, FnCode<'a>)>,
    unknown: Option<Vec<String>>,
    region: u32,
    readers: FxHashSet<NativeId>,
    old: FxHashSet<usize>,
    dirty: bool,
}

pub fn function_text(texts: &[String]) -> String {
    match texts.split_last() {
        Some((body, params)) => format!("{FUNCTION_HEAD}{}\n) {{\n{body}\n}}", params.join(",")),
        None => format!("{FUNCTION_HEAD}\n) {{\n\n}}"),
    }
}

pub fn run_sink<'a>(script: SinkScript<'a, 'a>, fuel: u64, heap: u64) -> SinkRun {
    let ledger = HeapLedger::new(heap);
    let mut interp = Interp::new(script.program.source_text, Budget::new(fuel));
    interp.set_heap(Some(ledger.clone()));
    let outcome = sunk(&mut interp, &script);
    let prefix_clean = interp.sink.as_ref().is_some_and(|state| !state.dirty);
    let steps = interp.budget.steps_used();
    drop(interp);
    SinkRun {
        outcome,
        prefix_clean,
        steps,
        heap: ledger.peak(),
    }
}

fn sunk<'a>(interp: &mut Interp<'a>, script: &SinkScript<'a, 'a>) -> SinkOutcome {
    let program = script.program;
    let Some(Statement::ExpressionStatement(last)) = program.body.get(script.sink) else {
        return SinkOutcome::Refused;
    };
    if !program.directives.is_empty() || super::lexical::refuse_program_dead_zones(program).is_err()
    {
        return SinkOutcome::Refused;
    }
    let state = SinkState::new(interp, script);
    interp.sink = Some(Box::new(state));
    let value = run_to_sink(interp, &program.body[..script.sink], &last.expression);
    interp.watch = Watch::Off;
    let unknown = interp.sink.as_mut().and_then(|state| state.unknown.take());
    match (value, unknown) {
        (_, Some(texts)) => SinkOutcome::Unknown(texts),
        (Ok(value), None) => reached(interp, &value).unwrap_or(SinkOutcome::Refused),
        (Err(_), None) => SinkOutcome::Refused,
    }
}

fn run_to_sink<'a>(
    interp: &mut Interp<'a>,
    prefix: &'a [Statement<'a>],
    sink: &'a Expression<'a>,
) -> Eval<'a, Value<'a>> {
    let env = interp.global_env();
    interp.hoist(prefix, &env)?;
    let Some(root) = crate::compile::program::root() else {
        return Err(Bail::Unsupported(&"a program root the compiler could not lay out").into());
    };
    mark_old(interp);
    interp.watch = Watch::Note;
    crate::compile::program::run_top_level(interp, (prefix, &root), &env, Value::Undefined)?;
    mark_old(interp);
    interp.watch = Watch::Confine;
    interp.eval_root_expression(sink)
}

fn mark_old(interp: &mut Interp<'_>) {
    let old = interp.realm.live_object_addresses();
    if let Some(state) = interp.sink.as_mut() {
        state.old = old;
    }
}

fn reached<'a>(interp: &Interp<'a>, value: &Value<'a>) -> Option<SinkOutcome> {
    let state = interp.sink.as_ref()?;
    let obj = value.as_object()?;
    let borrowed = obj.borrow();
    let ObjKind::Array(items) = &borrowed.kind else {
        return None;
    };
    let (callee, args) = items.split_first()?;
    let callee = callee.as_object()?;
    let callee = if state
        .function
        .as_ref()
        .is_some_and(|f| Rc::ptr_eq(f, callee))
    {
        SinkCallee::Function
    } else if state.eval.as_ref().is_some_and(|e| Rc::ptr_eq(e, callee)) {
        SinkCallee::Eval
    } else {
        return None;
    };
    let texts = strings(args)?;
    Some(SinkOutcome::Reached { callee, texts })
}

fn strings(values: &[Value<'_>]) -> Option<Vec<String>> {
    values
        .iter()
        .map(|value| match value {
            Value::Str(text) => text.as_string(),
            _ => None,
        })
        .collect()
}

impl<'a> SinkState<'a> {
    fn new(interp: &mut Interp<'a>, script: &SinkScript<'a, 'a>) -> Self {
        let root = interp.global_env();
        let mut global = |name: &'static str| {
            let name = interp.name_of(name);
            read_binding(&root, name, || Bail::unresolved_global(""))
                .ok()
                .and_then(|value| value.as_object().cloned())
        };
        let function = global("Function");
        let eval = global("eval");
        let boolean = global("Boolean");
        let constructors = LINKED_CONSTRUCTORS.map(|name| global(name).map(Value::Object));
        interp.realm.link_sink_constructors(constructors);
        let readers = reader_natives(interp);
        let pool = script
            .pool
            .iter()
            .map(|(text, function)| (text.clone(), FnCode::Function(function)))
            .collect();
        Self {
            function,
            boolean,
            eval,
            pool,
            unknown: None,
            region: script.region,
            readers,
            old: FxHashSet::default(),
            dirty: false,
        }
    }

    fn is_function(&self, obj: &ObjRef<'a>) -> bool {
        self.function.as_ref().is_some_and(|f| Rc::ptr_eq(f, obj))
    }

    fn is_old(&self, obj: &ObjRef<'a>) -> bool {
        self.old.contains(&(Rc::as_ptr(obj) as usize))
    }
}

fn reader_natives(interp: &Interp<'_>) -> FxHashSet<NativeId> {
    let protos = [
        interp.realm.array_prototype(),
        interp.realm.object_prototype(),
        interp.realm.function_prototype(),
    ];
    let mut ids = FxHashSet::default();
    for proto in protos.iter().flatten() {
        for name in READER_MEMBERS {
            let key = PropKey::Name(JsString::from_str(name));
            let Ok(Some(member)) = lookup_property(proto, &key) else {
                continue;
            };
            if let Some(id) = native_id(&member) {
                ids.insert(id);
            }
        }
    }
    ids
}

fn native_id(value: &Value<'_>) -> Option<NativeId> {
    match value.as_object()?.borrow().kind {
        ObjKind::Native(id, _) => Some(id),
        _ => None,
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn sink_boolean_member(&self, key: &PropKey) -> Option<Value<'a>> {
        let state = self.sink.as_ref()?;
        let PropKey::Name(name) = key else {
            return None;
        };
        if name.units() != CONSTRUCTOR_UNITS {
            return None;
        }
        state.boolean.clone().map(Value::Object)
    }

    pub(crate) fn sink_reads_hostile(&self, obj: &ObjRef<'a>) -> bool {
        self.sink.is_some() && matches!(obj.borrow().kind, ObjKind::Native(_, _))
    }

    pub(crate) fn sink_constructs(&self, obj: &ObjRef<'a>) -> bool {
        self.sink
            .as_ref()
            .is_some_and(|state| state.is_function(obj))
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn sink_function(&mut self, args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        let Some(texts) = strings(args) else {
            return Err(Bail::Unsupported(&OUTSIDE_POOL).into());
        };
        let text = function_text(&texts);
        let Some(state) = self.sink.as_mut() else {
            return Err(Bail::Unsupported(&OUTSIDE_POOL).into());
        };
        let Some(code) = state
            .pool
            .iter()
            .find_map(|(known, code)| (*known == text).then_some(*code))
        else {
            state.unknown = Some(texts);
            return Err(Bail::Unsupported(&OUTSIDE_POOL).into());
        };
        let env = self.global_env();
        let strict = std::mem::replace(&mut self.strict, false);
        let closure = self.make_closure(code, &env);
        self.strict = strict;
        Ok(closure)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn watched_native(&mut self, id: NativeId, this: &Value<'a>) -> Eval<'a, ()> {
        let Some(state) = self.sink.as_ref() else {
            return Err(Bail::Unsupported(&CONFINED_NATIVE).into());
        };
        let fresh = this.as_object().is_some_and(|obj| !state.is_old(obj));
        let admitted = self.realm.is_pure_native(id)
            || state.readers.contains(&id)
            || self.realm.is_locally_pure_native(id) && fresh;
        self.watched(admitted, &CONFINED_NATIVE)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn watched_closure(&mut self, closure: &Closure<'a>) -> Eval<'a, ()> {
        let start = match closure.code {
            FnCode::Function(function) => function.span().start,
            FnCode::Arrow(arrow) => arrow.span().start,
        };
        let inside = self
            .sink
            .as_ref()
            .is_some_and(|state| start >= state.region);
        self.watched(inside, &CONFINED_CLOSURE)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn watched_write(&mut self, obj: &ObjRef<'a>) {
        let old = self.sink.as_ref().is_some_and(|state| state.is_old(obj));
        if old {
            self.mark_dirty();
        }
    }

    fn watched(&mut self, admitted: bool, why: &'static &'static str) -> Eval<'a, ()> {
        match (admitted, self.watch) {
            (true, _) | (_, Watch::Off) => Ok(()),
            (false, Watch::Note) => {
                self.mark_dirty();
                Ok(())
            }
            (false, Watch::Confine) => Err(Bail::Unsupported(why).into()),
        }
    }

    fn mark_dirty(&mut self) {
        if let Some(state) = self.sink.as_mut() {
            state.dirty = true;
        }
    }
}
