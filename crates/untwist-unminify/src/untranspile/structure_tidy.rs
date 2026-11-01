use rustc_hash::FxHashMap;

use super::machine::Machine;
use super::structure::{Code, Fill, LabelId};

fn ends_abruptly(code: &[Code]) -> bool {
    match code.last() {
        Some(Code::Break(_) | Code::Continue(_) | Code::Return(..) | Code::Stop) => true,
        Some(Code::If {
            then, otherwise, ..
        }) => ends_abruptly(then) && ends_abruptly(otherwise),
        _ => false,
    }
}

fn flatten(code: &mut Vec<Code>) {
    let mut at = 0;
    while at < code.len() {
        let moved = match code.get_mut(at) {
            Some(Code::If {
                then,
                otherwise,
                negated,
                ..
            }) => {
                flatten(then);
                flatten(otherwise);
                if otherwise.is_empty() {
                    None
                } else if ends_abruptly(then) {
                    Some(std::mem::take(otherwise))
                } else if ends_abruptly(otherwise) && !then.is_empty() {
                    std::mem::swap(then, otherwise);
                    *negated = !*negated;
                    Some(std::mem::take(otherwise))
                } else {
                    None
                }
            }
            Some(Code::Loop(_, body) | Code::Block(_, body) | Code::Try { block: body, .. }) => {
                flatten(body);
                None
            }
            _ => None,
        };
        if let Some(Code::Try {
            handler, finalizer, ..
        }) = code.get_mut(at)
        {
            if let Some(handler) = handler {
                flatten(handler);
            }
            if let Some(finalizer) = finalizer {
                flatten(finalizer);
            }
        }
        if let Some(moved) = moved {
            let tail = code.split_off(at + 1);
            code.extend(moved);
            code.extend(tail);
        }
        at += 1;
    }
}

#[derive(Clone, Default)]
struct Exits {
    breaks: Vec<LabelId>,
    conts: Vec<LabelId>,
}

impl Exits {
    fn with_break(&self, label: LabelId) -> Exits {
        let mut next = self.clone();
        next.breaks.push(label);
        next
    }

    fn only_break(label: LabelId) -> Exits {
        Exits {
            breaks: vec![label],
            conts: Vec::new(),
        }
    }
}

fn strip(code: &mut Vec<Code>, tail: &Exits) {
    while let Some(last) = code.last() {
        let removable = match last {
            Code::Break(label) => tail.breaks.contains(label),
            Code::Continue(label) => tail.conts.contains(label),
            _ => false,
        };
        if !removable {
            break;
        }
        code.pop();
    }
    let count = code.len();
    for (at, item) in code.iter_mut().enumerate() {
        let local = if at + 1 == count {
            tail.clone()
        } else {
            Exits::default()
        };
        strip_inside(item, &local);
    }
}

fn strip_inside(item: &mut Code, tail: &Exits) {
    match item {
        Code::If {
            then, otherwise, ..
        } => {
            strip(then, tail);
            strip(otherwise, tail);
        }
        Code::Block(label, body) => strip(body, &tail.with_break(*label)),
        Code::Loop(label, body) => strip(
            body,
            &Exits {
                breaks: Vec::new(),
                conts: vec![*label],
            },
        ),
        Code::Try {
            label,
            block,
            handler,
            finalizer,
            ..
        } => {
            let flows = tail.with_break(*label);
            strip(block, &flows);
            if let Some(handler) = handler {
                strip(handler, &flows);
            }
            if let Some(finalizer) = finalizer {
                strip(finalizer, &Exits::default());
            }
        }
        _ => {}
    }
}

fn count_refs(code: &[Code], refs: &mut FxHashMap<LabelId, u32>) {
    for item in code {
        match item {
            Code::Break(label) | Code::Continue(label) => *refs.entry(*label).or_insert(0) += 1,
            Code::If {
                then, otherwise, ..
            } => {
                count_refs(then, refs);
                count_refs(otherwise, refs);
            }
            Code::Loop(_, body) | Code::While(_, _, _, body) | Code::Block(_, body) => {
                count_refs(body, refs);
            }
            Code::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                count_refs(block, refs);
                for part in [handler, finalizer].into_iter().flatten() {
                    count_refs(part, refs);
                }
            }
            _ => {}
        }
    }
}

fn splice_unused(code: &mut Vec<Code>, refs: &FxHashMap<LabelId, u32>) {
    let mut out = Vec::with_capacity(code.len());
    for mut item in std::mem::take(code) {
        match &mut item {
            Code::Block(label, body) if !refs.contains_key(label) => {
                splice_unused(body, refs);
                out.append(body);
                continue;
            }
            Code::If {
                then, otherwise, ..
            } => {
                splice_unused(then, refs);
                splice_unused(otherwise, refs);
            }
            Code::Loop(_, body) | Code::While(_, _, _, body) | Code::Block(_, body) => {
                splice_unused(body, refs);
            }
            Code::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                splice_unused(block, refs);
                for part in [handler, finalizer].into_iter().flatten() {
                    splice_unused(part, refs);
                }
            }
            _ => {}
        }
        out.push(item);
    }
    *code = out;
}

fn prune(code: &mut Vec<Code>, machine: &Machine) {
    code.retain(|item| {
        !matches!(item, Code::Node(node, _)
            if machine.node(*node).is_some_and(|node| node.body.is_empty()))
    });
    for item in code.iter_mut() {
        match item {
            Code::If {
                then, otherwise, ..
            } => {
                prune(then, machine);
                prune(otherwise, machine);
            }
            Code::Loop(_, body) | Code::While(_, _, _, body) | Code::Block(_, body) => {
                prune(body, machine);
            }
            Code::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                prune(block, machine);
                for part in [handler, finalizer].into_iter().flatten() {
                    prune(part, machine);
                }
            }
            _ => {}
        }
    }
}

fn leading_exit(body: &[Code], label: LabelId) -> bool {
    matches!(body.first(), Some(Code::If { fill: Fill::Nothing, then, otherwise, .. })
        if otherwise.is_empty() && matches!(then.as_slice(), [Code::Break(target)] if *target == label))
}

fn loop_tests(code: &mut [Code]) {
    for item in code.iter_mut() {
        match item {
            Code::Loop(label, body) if leading_exit(body, *label) => {
                let mut rest = body.split_off(1);
                loop_tests(&mut rest);
                if let Some(Code::If { test, negated, .. }) = body.pop() {
                    *item = Code::While(*label, test, !negated, rest);
                }
            }
            Code::If {
                then, otherwise, ..
            } => {
                loop_tests(then);
                loop_tests(otherwise);
            }
            Code::Loop(_, body) | Code::While(_, _, _, body) | Code::Block(_, body) => {
                loop_tests(body);
            }
            Code::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                loop_tests(block);
                for part in [handler, finalizer].into_iter().flatten() {
                    loop_tests(part);
                }
            }
            _ => {}
        }
    }
}

pub fn tidy(code: &mut Vec<Code>, machine: &Machine) -> FxHashMap<LabelId, u32> {
    prune(code, machine);
    flatten(code);
    strip(code, &Exits::only_break(0));
    loop_tests(code);
    let mut refs = FxHashMap::default();
    count_refs(code, &mut refs);
    splice_unused(code, &refs);
    refs
}
