use std::collections::VecDeque;

use oxc_ast::ast::Expression;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::aliased::{
    widened, BuiltinAliases, ANY, DANGER, FOREIGN, GLOBAL, HOSTILE, HOST_CODE, LOST,
    OBJECT_CONSTRUCTOR,
};
use super::aliased_flows::{merge_names, string_array, string_object, written_after_init, Target};

const BUDGET_STRIDE: usize = 1 << 10;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Read {
    Value(SymbolId),
    Return(Span),
    Strings(SymbolId),
    Table(SymbolId),
}

type Flow<'p> = (Target, &'p Expression<'p>);
type Install<'p> = (&'p Expression<'p>, Option<&'p Expression<'p>>);

struct Work {
    queue: VecDeque<usize>,
    queued: Vec<bool>,
    readers: FxHashMap<Read, Vec<usize>>,
    seen: FxHashSet<(Read, usize)>,
    popped: usize,
}

impl Work {
    fn all(count: usize) -> Self {
        Self {
            queue: (0..count).collect(),
            queued: vec![true; count],
            readers: FxHashMap::default(),
            seen: FxHashSet::default(),
            popped: 0,
        }
    }

    fn next(&mut self) -> Option<usize> {
        let at = self.queue.pop_front()?;
        self.queued[at] = false;
        self.popped += 1;
        Some(at)
    }

    fn over_budget(&self) -> bool {
        self.popped.is_multiple_of(BUDGET_STRIDE) && untwist_core::budget::check().is_err()
    }

    fn push(&mut self, at: usize) {
        if !self.queued[at] {
            self.queued[at] = true;
            self.queue.push_back(at);
        }
    }

    fn read_by(&mut self, at: usize, reads: impl Iterator<Item = Read>) {
        for read in reads {
            if self.seen.insert((read, at)) {
                self.readers.entry(read).or_default().push(at);
            }
        }
    }

    fn wake(&mut self, read: Read) {
        let readers = self.readers.get(&read).cloned().unwrap_or_default();
        for at in readers {
            self.push(at);
        }
    }

    fn wake_all(&mut self) {
        for at in 0..self.queued.len() {
            self.push(at);
        }
    }
}

fn evaluate<R>(
    aliases: &BuiltinAliases<'_>,
    work: &mut Work,
    at: usize,
    run: impl FnOnce() -> R,
) -> R {
    aliases.recording.set(true);
    let result = run();
    aliases.recording.set(false);
    let reads: Vec<Read> = aliases.reads.borrow_mut().drain(..).collect();
    work.read_by(at, reads.into_iter());
    result
}

fn flow_value(aliases: &BuiltinAliases<'_>, (target, expr): &Flow<'_>) -> u16 {
    let mut value = aliases.value(expr);
    if matches!(target, Target::Lost) {
        if let Some(function) = aliases.function_span(expr) {
            value |= aliases.returned(function);
        }
    }
    value
}

enum Woken {
    Nothing,
    One(Read),
    All,
}

fn merge_flow(aliases: &mut BuiltinAliases<'_>, target: &Target, value: u16) -> Woken {
    let own = merge_into(aliases, target, value);
    let exposed = match *target {
        Target::Symbol(symbol) | Target::Pattern(symbol) => aliases.parameters.contains(&symbol),
        _ => false,
    };
    if exposed {
        if let Woken::All = merge_into(aliases, &Target::Lost, flow_added(target, value)) {
            return Woken::All;
        }
    }
    own
}

fn flow_added(target: &Target, value: u16) -> u16 {
    match target {
        Target::Pattern(_) => ANY,
        Target::Lost => value & !(LOST | FOREIGN | HOSTILE | HOST_CODE),
        _ => value & !LOST,
    }
}

fn merge_into(aliases: &mut BuiltinAliases<'_>, target: &Target, value: u16) -> Woken {
    let (slot, woken) = match *target {
        Target::Symbol(symbol) | Target::Pattern(symbol) => (
            aliases.values.entry(symbol).or_insert(0),
            Woken::One(Read::Value(symbol)),
        ),
        Target::Lost => (&mut aliases.lost, Woken::All),
        Target::Return(function) => (
            aliases.returns.entry(function).or_insert(0),
            Woken::One(Read::Return(function)),
        ),
        Target::ThisArgument => (&mut aliases.this_bits, Woken::All),
    };
    let added = flow_added(target, value);
    let merged = *slot | added;
    if merged == *slot {
        return Woken::Nothing;
    }
    *slot = merged;
    woken
}

fn install_this(aliases: &mut BuiltinAliases<'_>, installs: &[Install<'_>]) -> bool {
    let mut changed = false;
    for (object, value) in installs {
        let installs_code = value.is_none_or(|value| aliases.function_span(value).is_some());
        let bits = widened(aliases.value(object)) & (DANGER | OBJECT_CONSTRUCTOR | GLOBAL);
        if installs_code && bits & !aliases.this_bits != 0 {
            aliases.this_bits |= bits;
            changed = true;
        }
    }
    changed
}

fn lose_everything(aliases: &mut BuiltinAliases<'_>, flows: &[Flow<'_>]) {
    for (target, _) in flows {
        match *target {
            Target::Symbol(symbol) | Target::Pattern(symbol) => {
                aliases.values.insert(symbol, ANY);
            }
            Target::Return(function) => {
                aliases.returns.insert(function, ANY);
            }
            Target::Lost | Target::ThisArgument => {}
        }
    }
    aliases.lost = ANY;
    aliases.this_bits = ANY;
}

pub(super) fn solve_bits(
    aliases: &mut BuiltinAliases<'_>,
    flows: &[Flow<'_>],
    installs: &[Install<'_>],
) {
    let mut work = Work::all(flows.len());
    loop {
        while let Some(at) = work.next() {
            if work.over_budget() {
                return lose_everything(aliases, flows);
            }
            let flow = &flows[at];
            let value = evaluate(aliases, &mut work, at, || flow_value(aliases, flow));
            if value == 0 {
                continue;
            }
            match merge_flow(aliases, &flow.0, value) {
                Woken::Nothing => {}
                Woken::One(read) => work.wake(read),
                Woken::All => work.wake_all(),
            }
        }
        if !install_this(aliases, installs) {
            return;
        }
        work.wake_all();
    }
}

type ArrayInit<'p> = (SymbolId, &'p Expression<'p>);

fn solve_table(
    aliases: &mut BuiltinAliases<'_>,
    work: &mut Work,
    at: usize,
    (symbol, init): &ArrayInit<'_>,
) {
    let names = evaluate(aliases, work, at, || string_array(aliases, init));
    if let Some(names) = names {
        let known = aliases.string_arrays.get(symbol).map_or(0, Vec::len);
        if names.len() != known {
            aliases.string_arrays.insert(*symbol, names);
            work.wake(Read::Table(*symbol));
        }
    }
    let properties = evaluate(aliases, work, at, || string_object(aliases, init));
    if let Some(properties) = properties {
        let size = |map: &FxHashMap<String, Vec<String>>| map.values().map(Vec::len).sum::<usize>();
        let known = aliases.string_objects.get(symbol).map_or(0, size);
        if size(&properties) != known {
            aliases.string_objects.insert(*symbol, properties);
            work.wake(Read::Table(*symbol));
        }
    }
}

fn solve_string_flow(
    aliases: &mut BuiltinAliases<'_>,
    work: &mut Work,
    at: usize,
    (target, expr): &Flow<'_>,
) {
    let symbol = match target {
        Target::Symbol(symbol) => *symbol,
        Target::Pattern(symbol) => {
            if aliases.unknown_strings.insert(*symbol) {
                work.wake(Read::Strings(*symbol));
            }
            return;
        }
        _ => return,
    };
    if aliases.unknown_strings.contains(&symbol) {
        return;
    }
    let previous = aliases.strings.get(&symbol).cloned().unwrap_or_default();
    let size = previous.len();
    let names = evaluate(aliases, work, at, || aliases.key_set(expr));
    match names.and_then(|names| merge_names(previous, names)) {
        Some(names) if names.len() != size => {
            aliases.strings.insert(symbol, names);
            work.wake(Read::Strings(symbol));
        }
        Some(_) => {}
        None => {
            aliases.strings.remove(&symbol);
            aliases.unknown_strings.insert(symbol);
            work.wake(Read::Strings(symbol));
        }
    }
}

pub(super) fn solve_strings(
    aliases: &mut BuiltinAliases<'_>,
    flows: &[Flow<'_>],
    (arrays, escaped): (&[ArrayInit<'_>], &FxHashSet<SymbolId>),
) {
    let arrays: Vec<&ArrayInit<'_>> = arrays
        .iter()
        .filter(|(symbol, _)| {
            !escaped.contains(symbol) && !written_after_init(aliases.scoping, *symbol)
        })
        .collect();
    let mut work = Work::all(arrays.len() + flows.len());
    while let Some(at) = work.next() {
        if work.over_budget() {
            return lose_strings(aliases, flows);
        }
        match arrays.get(at) {
            Some(array) => solve_table(aliases, &mut work, at, array),
            None => solve_string_flow(aliases, &mut work, at, &flows[at - arrays.len()]),
        }
    }
}

fn lose_strings(aliases: &mut BuiltinAliases<'_>, flows: &[Flow<'_>]) {
    aliases.strings.clear();
    aliases.string_arrays.clear();
    aliases.string_objects.clear();
    for (target, _) in flows {
        if let Target::Symbol(symbol) | Target::Pattern(symbol) = target {
            aliases.unknown_strings.insert(*symbol);
        }
    }
}
