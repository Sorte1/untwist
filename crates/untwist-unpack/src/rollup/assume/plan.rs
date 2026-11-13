use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::literal::is_pure_value;

use super::outer::Outer;
#[cfg(feature = "boundaries")]
use super::runs::{runs, Input};
#[cfg(feature = "boundaries")]
use super::setters::writes;
use super::shared::{prefix_end, Absorb, Free};
use crate::rollup::scan::Facts;
use crate::rollup::wrappers::Group;

const PROTO: &str = "__proto__";
const LATE: usize = usize::MAX;
const SHARED: &str = "shared";

pub struct Boundary {
    pub members: Vec<u32>,
    pub stem: String,
}

pub struct Plan {
    pub parts: Vec<Boundary>,
    pub part_of: Vec<Option<usize>>,
    pub runs: Vec<Run>,
    pub run_of: Vec<Option<usize>>,
    pub run_setters: Vec<RunSetter>,
    pub shell: Vec<String>,
    pub setters: Vec<String>,
}

pub struct Run {
    pub members: Vec<u32>,
    pub stem: String,
}

pub struct RunSetter {
    pub name: String,
    pub run: usize,
    pub writes: Vec<(u32, u32, bool)>,
}

pub fn boundaries(groups: Vec<Group>, libraries: Vec<Boundary>) -> Vec<Boundary> {
    let mut found: Vec<Boundary> = groups
        .into_iter()
        .map(|group| {
            let mut members = group.members;
            members.extend(group.holder);
            members.sort_unstable();
            members.dedup();
            Boundary {
                members,
                stem: group.stem,
            }
        })
        .chain(libraries)
        .collect();
    let mut seen: FxHashMap<u32, usize> = FxHashMap::default();
    let mut clashing = FxHashSet::default();
    for (index, boundary) in found.iter().enumerate() {
        for &member in &boundary.members {
            if let Some(&other) = seen.get(&member) {
                clashing.insert(other);
                clashing.insert(index);
            }
            seen.insert(member, index);
        }
    }
    let mut index = 0;
    found.retain(|_| {
        index += 1;
        !clashing.contains(&(index - 1))
    });
    found
}

pub(super) fn quiet_value(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            let named = property.key.static_name().is_some_and(|name| name != PROTO);
            property.kind == PropertyKind::Init
                && !property.computed
                && !property.method
                && !property.shorthand
                && named
                && quiet_value(&property.value)
        }),
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            element.as_expression().map_or(
                matches!(element, ArrayExpressionElement::Elision(_)),
                quiet_value,
            )
        }),
        other => is_pure_value(other),
    }
}

pub(super) fn initialised(item: &Statement<'_>) -> Option<bool> {
    match item {
        Statement::FunctionDeclaration(function) => function.body.is_some().then_some(false),
        Statement::VariableDeclaration(declaration)
            if declaration.kind == VariableDeclarationKind::Var =>
        {
            let mut init = false;
            for declarator in &declaration.declarations {
                declarator.id.get_binding_identifier()?;
                if let Some(value) = &declarator.init {
                    quiet_value(value).then_some(())?;
                    init = true;
                }
            }
            Some(init)
        }
        _ => None,
    }
}

fn references(facts: &Facts, n: usize) -> Vec<Vec<u32>> {
    let mut refs: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &(item, symbol) in &facts.uses {
        refs[item as usize].push(facts.owner[symbol.index()]);
    }
    for targets in &mut refs {
        targets.sort_unstable();
        targets.dedup();
    }
    refs
}

fn earliest(refs: &[Vec<u32>], items: &[Statement<'_>]) -> Vec<usize> {
    let mut time = vec![LATE; items.len()];
    let mut stack = Vec::new();
    for (start, item) in items.iter().enumerate() {
        if initialised(item).is_some() {
            continue;
        }
        stack.push(start);
        while let Some(at) = stack.pop() {
            for &target in &refs[at] {
                if time[target as usize] == LATE {
                    time[target as usize] = start;
                    stack.push(target as usize);
                }
            }
        }
    }
    time
}

struct Checks<'f> {
    facts: &'f Facts,
    part_of: Vec<Option<usize>>,
    unclean: FxHashSet<String>,
    clashing: Vec<bool>,
}

fn clashing(scoping: &Scoping, facts: &Facts, n: usize, outer: Option<&Outer>) -> Vec<bool> {
    let mut found = vec![false; n];
    let Some(outer) = outer.filter(|outer| !outer.names.is_empty()) else {
        return found;
    };
    for (symbol, &owner) in facts.owner.iter().enumerate() {
        let Some(slot) = found.get_mut(owner as usize) else {
            continue;
        };
        let symbol = oxc_syntax::symbol::SymbolId::from_usize(symbol);
        *slot |= outer.names.contains(scoping.symbol_name(symbol));
    }
    found
}

impl Checks<'_> {
    fn movable(&self, at: usize) -> bool {
        let item = at as u32;
        let spans_ok = !self
            .facts
            .spans
            .iter()
            .any(|&(low, high)| low == item || high == item);
        let writes_ok = self
            .facts
            .writes
            .iter()
            .all(|&(writer, owner)| owner != item || writer == item);
        let shell_ok =
            self.facts.shell.iter().all(|(user, name, write)| {
                *user != item || (!write && !self.unclean.contains(name))
            });
        let own = !self.facts.pinned[at] && !self.facts.dropped[at] && !self.clashing[at];
        own && spans_ok && writes_ok && shell_ok
    }

    fn eligible(&self, items: &[Statement<'_>], time: &[usize], free: &Free<'_>) -> Vec<bool> {
        let prefix = prefix_end(items, free);
        (0..items.len())
            .map(|at| {
                let untouched = time[at] == LATE || time[at] > at;
                let shape = match initialised(&items[at]) {
                    Some(init) => !init || untouched,
                    None => free.probe_var(&items[at]) == Some(false) && at < prefix && untouched,
                };
                self.part_of[at].is_none() && shape && self.movable(at)
            })
            .collect()
    }

    fn static_ok(&self, items: &[Statement<'_>], time: &[usize], boundary: &Boundary) -> bool {
        let inside = |item: u32| boundary.members.contains(&item);
        let members_ok = boundary.members.iter().all(|&member| {
            let at = member as usize;
            let Some(init) = initialised(&items[at]) else {
                return false;
            };
            let untouched = !init || time[at] == LATE || time[at] > at;
            untouched && !self.facts.pinned[at] && !self.facts.dropped[at] && !self.clashing[at]
        });
        let spans_ok = !self
            .facts
            .spans
            .iter()
            .any(|&(low, high)| inside(low) || inside(high));
        let writes_ok = self
            .facts
            .writes
            .iter()
            .all(|&(writer, owner)| !inside(owner) || inside(writer));
        let shell_ok =
            self.facts.shell.iter().all(|(item, name, write)| {
                !inside(*item) || (!write && !self.unclean.contains(name))
            });
        members_ok && spans_ok && writes_ok && shell_ok
    }
}

fn unclean(outer: Option<&Outer>, facts: &Facts) -> FxHashSet<String> {
    let mut found: FxHashSet<String> = outer.map(|outer| outer.unclean.clone()).unwrap_or_default();
    for (_, name, write) in &facts.shell {
        if *write {
            found.insert(name.clone());
        }
    }
    found
}

fn settle(facts: &Facts, part_of: &mut [Option<usize>], alive: &mut [bool]) {
    loop {
        let mut changed = false;
        for &(item, symbol) in &facts.uses {
            let Some(part) = part_of[item as usize] else {
                continue;
            };
            let owner = facts.owner[symbol.index()] as usize;
            let reaches_entry = part_of[owner].is_none();
            if alive[part] && reaches_entry {
                alive[part] = false;
                changed = true;
            }
        }
        for slot in part_of.iter_mut() {
            if slot.is_some_and(|part| !alive[part]) {
                *slot = None;
            }
        }
        if !changed {
            return;
        }
    }
}

type Runs = (Vec<Run>, Vec<Option<usize>>, Vec<RunSetter>);

#[cfg(not(feature = "boundaries"))]
fn entry_runs(
    _scoping: &Scoping,
    items: &[Statement<'_>],
    _facts: &Facts,
    _refs: (&[Vec<u32>], &[Option<usize>]),
    _clashing: &[bool],
) -> Runs {
    (Vec::new(), vec![None; items.len()], Vec::new())
}

#[cfg(feature = "boundaries")]
fn entry_runs(
    scoping: &Scoping,
    items: &[Statement<'_>],
    facts: &Facts,
    (refs, part_of): (&[Vec<u32>], &[Option<usize>]),
    clashing: &[bool],
) -> Runs {
    let mut held: Vec<bool> = (0..items.len())
        .map(|at| facts.pinned[at] || clashing[at])
        .collect();
    for (item, _, _) in &facts.shell {
        held[*item as usize] = true;
    }
    let entry = (0..items.len() as u32)
        .filter(|&at| part_of[at as usize].is_none() && !facts.dropped[at as usize])
        .collect();
    let found_writes = writes(scoping, items, facts);
    let input = Input {
        entry,
        fixed: &found_writes.fixed,
        refs,
        held: &held,
    };
    let found = runs(scoping, items, facts, &input);
    let mut run_of = vec![None; items.len()];
    for (index, run) in found.iter().enumerate() {
        for &member in &run.members {
            run_of[member as usize] = Some(index);
        }
    }
    let module = |item: u32| (part_of[item as usize], run_of[item as usize]);
    let mut setters: Vec<RunSetter> = Vec::new();
    let mut by_symbol: FxHashMap<oxc_syntax::symbol::SymbolId, usize> = FxHashMap::default();
    for write in &found_writes.plain {
        if module(write.item) == module(write.owner) {
            continue;
        }
        let (None, Some(run)) = module(write.owner) else {
            return (Vec::new(), vec![None; items.len()], Vec::new());
        };
        let slot = *by_symbol.entry(write.symbol).or_insert_with(|| {
            setters.push(RunSetter {
                name: scoping.symbol_name(write.symbol).to_string(),
                run,
                writes: Vec::new(),
            });
            setters.len() - 1
        });
        setters[slot]
            .writes
            .push((write.item, write.at, write.named));
    }
    (found, run_of, setters)
}

pub fn plan(
    scoping: &Scoping,
    items: &[Statement<'_>],
    facts: &Facts,
    mut found: Vec<Boundary>,
    outer: Option<&Outer>,
) -> Option<Plan> {
    let refs = references(facts, items.len());
    let time = earliest(&refs, items);
    let mut checks = Checks {
        facts,
        part_of: vec![None; items.len()],
        unclean: unclean(outer, facts),
        clashing: clashing(scoping, facts, items.len(), outer),
    };
    let mut alive: Vec<bool> = found
        .iter()
        .map(|boundary| checks.static_ok(items, &time, boundary))
        .collect();
    for (index, boundary) in found.iter().enumerate() {
        for &member in &boundary.members {
            checks.part_of[member as usize] = alive[index].then_some(index);
        }
    }
    let names = outer.map(|outer| outer.names.clone()).unwrap_or_default();
    let free = Free {
        scoping,
        outer: &names,
    };
    let absorb = Absorb {
        refs: &refs,
        eligible: checks.eligible(items, &time, &free),
    };
    let shared = absorb.closure(&checks.part_of);
    if !shared.is_empty() {
        for &member in &shared {
            checks.part_of[member as usize] = Some(found.len());
        }
        found.push(Boundary {
            members: shared,
            stem: SHARED.to_string(),
        });
        alive.push(true);
    }
    settle(facts, &mut checks.part_of, &mut alive);
    let mut renumber = vec![None; found.len()];
    let mut parts = Vec::new();
    for (index, boundary) in found.into_iter().enumerate() {
        if alive[index] {
            renumber[index] = Some(parts.len());
            parts.push(boundary);
        }
    }
    let part_of: Vec<Option<usize>> = checks
        .part_of
        .iter()
        .map(|slot| slot.and_then(|part| renumber[part]))
        .collect();
    let (runs, run_of, run_setters) =
        entry_runs(scoping, items, facts, (&refs, &part_of), &checks.clashing);
    if parts.is_empty() && runs.is_empty() {
        return None;
    }
    let shell: Vec<String> = facts
        .shell
        .iter()
        .filter(|(item, _, _)| part_of[*item as usize].is_some())
        .map(|(_, name, _)| name.clone())
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();
    let setters = shell
        .iter()
        .filter(|name| outer.is_some_and(|outer| outer.assigned.contains(*name)))
        .cloned()
        .collect();
    Some(Plan {
        parts,
        part_of,
        runs,
        run_of,
        run_setters,
        shell,
        setters,
    })
}
