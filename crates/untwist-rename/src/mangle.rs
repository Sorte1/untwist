use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{BindingRole, Collected, InitKind, InitShape};
use crate::generate::{title_case, to_identifier};
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::reserved::Excluded;
use crate::RenameConfig;

const READ_DEPTH: usize = 4;

pub struct Mangle {
    extended: bool,
    done: bool,
}

impl Mangle {
    pub fn new(config: RenameConfig) -> Self {
        Self {
            extended: config.extended,
            done: false,
        }
    }
}

pub fn boxed<'a>(config: RenameConfig) -> Box<dyn Pass<'a> + 'a> {
    Box::new(Mangle::new(config))
}

impl<'a> Pass<'a> for Mangle {
    fn name(&self) -> &'static str {
        "mangle"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let extended = self.extended;
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Report,
            OuterRule::ProgramWide,
            move |_, collected, scoping| decide(collected, scoping, extended),
        );
    }
}

const FAST: Path = Path {
    stops_at_exhaustion: true,
    skips_handed_out: true,
};

#[derive(Clone, Copy)]
struct Path {
    stops_at_exhaustion: bool,
    skips_handed_out: bool,
}

pub fn decide(collected: &Collected, scoping: &Scoping, extended: bool) -> RenamePlan {
    decide_sites(collected, scoping, extended, FAST)
}

#[doc(hidden)]
pub fn decide_every_site_for_tests(
    collected: &Collected,
    scoping: &Scoping,
    extended: bool,
) -> RenamePlan {
    let path = Path {
        stops_at_exhaustion: false,
        ..FAST
    };
    decide_sites(collected, scoping, extended, path)
}

#[doc(hidden)]
pub fn decide_by_rescan_for_tests(
    collected: &Collected,
    scoping: &Scoping,
    extended: bool,
) -> RenamePlan {
    let path = Path {
        skips_handed_out: false,
        ..FAST
    };
    decide_sites(collected, scoping, extended, path)
}

fn decide_sites(
    collected: &Collected,
    scoping: &Scoping,
    extended: bool,
    path: Path,
) -> RenamePlan {
    let mut decider = Decider::new(collected, scoping, extended, path);
    for index in 0..collected.sites.len() {
        decider.decide_site(index, 0);
    }
    match decider.exhausted {
        Some(name) => aborted(&name),
        None => decider.oracle.into_plan(),
    }
}

fn aborted(name: &str) -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(format!(
        "ran out of names for {name}; leaving the program unrenamed"
    ));
    plan
}

struct Decider<'d> {
    collected: &'d Collected,
    scoping: &'d Scoping,
    oracle: NameOracle<'d>,
    sites_of: FxHashMap<SymbolId, Vec<usize>>,
    decided: Vec<bool>,
    reading: Vec<SymbolId>,
    reads_ahead: bool,
    exhausted: Option<String>,
    path: Path,
}

impl<'d> Decider<'d> {
    fn new(collected: &'d Collected, scoping: &'d Scoping, extended: bool, path: Path) -> Self {
        let mut sites_of: FxHashMap<SymbolId, Vec<usize>> = FxHashMap::default();
        for (index, site) in collected.sites.iter().enumerate() {
            sites_of.entry(site.symbol).or_default().push(index);
        }
        let mut decider = Self {
            collected,
            scoping,
            oracle: NameOracle::new(scoping, collected),
            sites_of,
            decided: vec![false; collected.sites.len()],
            reading: Vec::new(),
            reads_ahead: extended,
            exhausted: None,
            path,
        };
        if extended {
            decider.release_original_names();
        }
        decider
    }

    fn release_original_names(&mut self) {
        let renamed: FxHashSet<SymbolId> = self
            .collected
            .sites
            .iter()
            .filter(|site| self.renames(site.symbol, &site.role))
            .map(|site| site.symbol)
            .collect();
        let mut kept = Excluded::from_scoping(self.scoping);
        for symbol in self.scoping.symbol_ids() {
            if !renamed.contains(&symbol) {
                kept.insert(self.scoping.symbol_name(symbol));
            }
        }
        self.oracle = NameOracle::keeping(self.scoping, self.collected, kept);
        self.oracle.defer(renamed);
    }

    fn renames(&self, symbol: SymbolId, role: &BindingRole) -> bool {
        !self.collected.pinned.contains(&symbol) && self.infer_name(role).is_some()
    }

    fn decide_site(&mut self, index: usize, depth: usize) {
        if self.path.stops_at_exhaustion && self.exhausted.is_some() {
            return;
        }
        let Some(slot) = self.decided.get_mut(index) else {
            return;
        };
        if std::mem::replace(slot, true) {
            return;
        }
        let collected = self.collected;
        let Some(site) = collected.sites.get(index) else {
            return;
        };
        if collected.pinned.contains(&site.symbol) {
            return;
        }
        let mark = self.reading.len();
        if let (true, BindingRole::Variable(shape)) = (self.reads_ahead, &site.role) {
            self.reading.push(site.symbol);
            self.decide_reads(shape, depth);
        }
        let base = self.infer_name(&site.role);
        self.reading.truncate(mark);
        let Some(base) = base else {
            return;
        };
        let taken = if self.path.skips_handed_out {
            self.oracle.take_uid(site.symbol, &base)
        } else {
            self.oracle.take_uid_by_rescan_for_tests(site.symbol, &base)
        };
        if taken.is_none() {
            self.exhausted
                .get_or_insert_with(|| self.scoping.symbol_name(site.symbol).to_string());
        }
    }

    fn decide_reads(&mut self, shape: &InitShape, depth: usize) {
        for symbol in read_symbols(shape) {
            self.decide_symbol(symbol, depth + 1);
        }
    }

    fn decide_symbol(&mut self, symbol: SymbolId, depth: usize) {
        if self.collected.pinned.contains(&symbol)
            || self.oracle.is_assigned(symbol)
            || self.reading.contains(&symbol)
        {
            return;
        }
        let Some(indices) = self.sites_of.get(&symbol).cloned() else {
            return;
        };
        if depth > READ_DEPTH {
            self.reading.push(symbol);
            return;
        }
        for index in indices {
            self.decide_site(index, depth);
        }
    }

    fn infer_name(&self, role: &BindingRole) -> Option<String> {
        match role {
            BindingRole::Class => Some("C".to_string()),
            BindingRole::Function => Some("f".to_string()),
            BindingRole::Param => Some("p".to_string()),
            BindingRole::CatchParam => Some("e".to_string()),
            BindingRole::PatternElement => Some("v".to_string()),
            BindingRole::Variable(shape) if self.calls_require(shape) => {
                shape.name.as_deref().map(to_identifier)
            }
            BindingRole::Variable(shape) => {
                let suffix = self.expression_name(shape).unwrap_or_default();
                Some(format!("v{}", title_case(&suffix)))
            }
            _ => None,
        }
    }

    fn calls_require(&self, shape: &InitShape) -> bool {
        shape.kind == InitKind::Require
            && shape
                .callee
                .as_deref()
                .and_then(|callee| self.resolved_name(callee))
                .is_some_and(|name| name == "require")
    }

    fn expression_name(&self, shape: &InitShape) -> Option<String> {
        match shape.kind {
            InitKind::Identifier => Some(self.resolved_name(shape).unwrap_or_default()),
            InitKind::Function => Some(self.resolved_name(shape).unwrap_or_else(|| "f".into())),
            InitKind::Class => Some(self.resolved_name(shape).unwrap_or_else(|| "C".into())),
            InitKind::Call | InitKind::Require => self.expression_name(shape.callee.as_deref()?),
            InitKind::This => Some("this".to_string()),
            InitKind::Number => shape.name.as_ref().map(|value| format!("LN{value}")),
            InitKind::String => shape.name.as_ref().map(|value| {
                let cleaned: String = title_case(value).chars().take(20).collect();
                format!("LS{cleaned}")
            }),
            InitKind::Object => Some("O".to_string()),
            InitKind::Array => Some("A".to_string()),
            _ => None,
        }
    }

    fn resolved_name(&self, shape: &InitShape) -> Option<String> {
        let Some(symbol) = shape.symbol else {
            return shape.name.clone();
        };
        if self.reading.contains(&symbol) {
            return Some(String::new());
        }
        Some(self.oracle.current_name(symbol).to_string())
    }
}

fn read_symbols(shape: &InitShape) -> Vec<SymbolId> {
    match shape.kind {
        InitKind::Identifier | InitKind::Function | InitKind::Class => {
            shape.symbol.into_iter().collect()
        }
        InitKind::Call | InitKind::Require => shape
            .callee
            .as_deref()
            .map(read_symbols)
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
