use std::cell::RefCell;
use std::rc::Rc;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::var_function_or_declaration;
use untwist_core::ast_util::rename::{taken_names, Renames};
use untwist_core::pass::Cx;
use untwist_core::semantic::program_is_strict;

use crate::notes;

pub use untwist_core::machinery_name::{
    decoder_index, is_machinery_name, may_name_machinery, string_array_unit, STRING_ARRAY_NAME,
};
use untwist_core::machinery_name::{ARRAY_PREFIX, DECODE_PREFIX};

pub fn string_array_name(unit: usize) -> String {
    if unit == 0 {
        STRING_ARRAY_NAME.to_string()
    } else {
        format!("{ARRAY_PREFIX}{unit}__")
    }
}

pub fn decoder_name(index: usize) -> String {
    format!("{DECODE_PREFIX}{index}__")
}

pub fn original_name<'s>(
    source_text: &'s str,
    scoping: &Scoping,
    symbol: SymbolId,
) -> Option<&'s str> {
    let span = scoping.symbol_span(symbol);
    source_text.get(span.start as usize..span.end as usize)
}

pub type RefusedByDecode = Rc<RefCell<FxHashSet<Span>>>;

pub fn unstamp_names<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    symbols: &[(SymbolId, String)],
) {
    let mut renames = Renames::new();
    for (symbol, name) in symbols {
        renames.insert(*symbol, Ident::from_str_in(name, &*cx));
    }
    let count = renames.apply(program, cx.scoping());
    cx.changed_by(count);
}

fn name_unit_counts(unstamp: &[Vec<(SymbolId, String)>]) -> FxHashMap<&str, usize> {
    let mut counts: FxHashMap<&str, usize> = FxHashMap::default();
    for symbols in unstamp {
        let names: FxHashSet<&str> = symbols.iter().map(|(_, name)| name.as_str()).collect();
        for name in names {
            *counts.entry(name).or_default() += 1;
        }
    }
    counts
}

pub fn unstamp_blocked(unstamp: &[Vec<(SymbolId, String)>], scoping: &Scoping) -> Vec<bool> {
    let counts = name_unit_counts(unstamp);
    let taken = std::cell::OnceCell::new();
    unstamp
        .iter()
        .map(|symbols| {
            symbols.iter().any(|(_, name)| {
                taken
                    .get_or_init(|| taken_names(scoping))
                    .contains(name.as_str())
                    || counts.get(name.as_str()).is_some_and(|n| *n > 1)
            })
        })
        .collect()
}

#[cold]
#[inline(never)]
pub fn unstamp_block_note(symbols: &[(SymbolId, String)], scoping: &Scoping) -> &'static str {
    let unresolved = scoping.root_unresolved_references();
    if symbols
        .iter()
        .any(|(_, name)| unresolved.contains_key(name.as_str()))
    {
        notes::UNSTAMP_FREE_REFERENCE
    } else {
        notes::UNSTAMP_BLOCKED
    }
}

#[derive(Default)]
struct ReferencedUnits {
    units: FxHashSet<usize>,
}

impl<'a> VisitJs<'a> for ReferencedUnits {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(unit) = string_array_unit(it.name.as_str()) {
            self.units.insert(unit);
        }
    }
}

pub fn referenced_units(function: &Function<'_>) -> FxHashSet<usize> {
    let mut probe = ReferencedUnits::default();
    if let Some(body) = function.body.as_ref() {
        probe.visit_function_body(body);
    }
    probe.units
}

pub fn owning_unit(function: &Function<'_>) -> Option<usize> {
    let mut units = referenced_units(function).into_iter();
    let unit = units.next()?;
    units.next().is_none().then_some(unit)
}

pub struct IndexAllocator {
    taken: FxHashSet<usize>,
    next: usize,
}

impl IndexAllocator {
    pub fn decoders(scoping: &Scoping) -> Self {
        Self::over(scoping, decoder_index)
    }

    pub fn string_arrays(scoping: &Scoping) -> Self {
        Self::over(scoping, string_array_unit)
    }

    fn over(scoping: &Scoping, number: fn(&str) -> Option<usize>) -> Self {
        let bound = scoping
            .symbol_ids()
            .filter_map(|symbol| number(scoping.symbol_name(symbol)));
        let free = scoping
            .root_unresolved_references()
            .keys()
            .filter_map(|name| number(name.as_str()));
        Self {
            taken: bound.chain(free).collect(),
            next: 0,
        }
    }

    pub fn allocate(&mut self) -> usize {
        while self.taken.contains(&self.next) {
            self.next += 1;
        }
        self.taken.insert(self.next);
        self.next
    }
}

pub fn declaration_symbol(stmt: &Statement<'_>) -> Option<SymbolId> {
    match stmt {
        Statement::FunctionDeclaration(function) => function.id.as_ref()?.symbol_id.get(),
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            id.symbol_id.get()
        }
        _ => None,
    }
}

pub struct StampedFn<'p> {
    pub name: &'p str,
    pub number: usize,
    pub statement: Span,
    pub function: &'p Function<'p>,
    pub symbol: Option<SymbolId>,
}

#[derive(Default)]
pub struct Stamped<'p> {
    pub arrays: Vec<StampedFn<'p>>,
    pub decoders: Vec<StampedFn<'p>>,
    pub incomplete: bool,
    pub top_level: TopLevel,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TopLevel {
    pub sorted: bool,
    pub strict: bool,
}

impl<'p> Stamped<'p> {
    pub fn is_empty(&self) -> bool {
        self.arrays.is_empty() && self.decoders.is_empty()
    }

    pub fn array(&self, unit: usize) -> Option<&StampedFn<'p>> {
        self.arrays.iter().find(|entry| entry.number == unit)
    }

    pub fn units(&self) -> Vec<usize> {
        self.arrays.iter().map(|entry| entry.number).collect()
    }

    pub fn owners(&self) -> Vec<Option<usize>> {
        let units = self.units();
        self.decoders
            .iter()
            .map(|decoder| owning_unit(decoder.function).filter(|unit| units.contains(unit)))
            .collect()
    }
}

pub fn declared_name<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    match stmt {
        Statement::FunctionDeclaration(function) => Some(function.id.as_ref()?.name.as_str()),
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            Some(id.name.as_str())
        }
        _ => None,
    }
}

#[derive(Default)]
struct Collector<'p> {
    arrays: Vec<StampedFn<'p>>,
    decoders: Vec<StampedFn<'p>>,
    incomplete: bool,
}

impl<'p> VisitJs<'p> for Collector<'p> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        let statement: &'p Statement<'p> = self.alloc(it);
        match var_function_or_declaration(statement) {
            Some(declaration) => {
                let make = |number| StampedFn {
                    name: declaration.name,
                    number,
                    statement: statement.span(),
                    function: declaration.function,
                    symbol: declaration_symbol(statement),
                };
                if let Some(unit) = string_array_unit(declaration.name) {
                    self.arrays.push(make(unit));
                } else if let Some(index) = decoder_index(declaration.name) {
                    self.decoders.push(make(index));
                }
            }
            None => {
                if declared_name(statement).is_some_and(is_machinery_name) {
                    self.incomplete = true;
                }
            }
        }
        walk_js::walk_statement(self, it);
    }
}

fn ordered<'p>(mut entries: Vec<StampedFn<'p>>) -> Vec<StampedFn<'p>> {
    entries.sort_by_key(|entry| entry.number);
    entries
}

pub fn binds_machinery_name(scoping: &Scoping) -> bool {
    scoping
        .symbol_names()
        .any(|name| may_name_machinery(name) && is_machinery_name(name))
}

pub fn is_unedited(cx: &Cx<'_, '_>) -> bool {
    cx.is_program_unchanged_this_traversal() && *cx.changes == 0
}

pub fn find_stamped_named<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    unedited: bool,
) -> Stamped<'p> {
    if unedited && !binds_machinery_name(scoping) {
        return Stamped::default();
    }
    find_stamped(program)
}

pub fn find_stamped<'p>(program: &'p Program<'p>) -> Stamped<'p> {
    let mut collector = Collector::default();
    collector.visit_program(program);
    Stamped {
        arrays: ordered(collector.arrays),
        decoders: ordered(collector.decoders),
        incomplete: collector.incomplete,
        top_level: TopLevel {
            sorted: top_level_sorted(program),
            strict: program_is_strict(program),
        },
    }
}

fn top_level_sorted(program: &Program<'_>) -> bool {
    program
        .body
        .iter()
        .all(|statement| statement.span().start < statement.span().end)
        && program
            .body
            .windows(2)
            .all(|pair| pair[0].span().end <= pair[1].span().start)
}

#[derive(Default)]
struct ArrayProbe {
    found: bool,
}

impl<'a> VisitJs<'a> for ArrayProbe {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.found {
            return;
        }
        if var_function_or_declaration(it)
            .is_some_and(|declaration| string_array_unit(declaration.name).is_some())
        {
            self.found = true;
            return;
        }
        walk_js::walk_statement(self, it);
    }
}

struct StrictAt {
    target: Span,
    strict: bool,
    found: Option<bool>,
}

impl StrictAt {
    fn holds(&self, span: Span) -> bool {
        self.found.is_none() && span.start <= self.target.start && self.target.end <= span.end
    }
}

impl<'a> VisitJs<'a> for StrictAt {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if it.span() == self.target {
            self.found.get_or_insert(self.strict);
        } else if self.holds(it.span()) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.holds(it.span()) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.holds(it.span) {
            self.strict = self.strict || it.has_use_strict_directive();
            walk_js::walk_function(self, it, flags);
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if self.holds(it.span) {
            self.strict = self.strict || it.body.has_use_strict_directive();
            walk_js::walk_arrow_function_expression(self, it);
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if self.holds(it.span) {
            self.strict = true;
            walk_js::walk_class(self, it);
        }
    }
}

pub fn context_is_strict_in(program: &Program<'_>, statement: Span, top: TopLevel) -> bool {
    if !top.sorted || statement.start >= statement.end {
        return context_is_strict(program, statement);
    }
    let after = program
        .body
        .partition_point(|candidate| candidate.span().start <= statement.start);
    let Some(candidate) = after.checked_sub(1).and_then(|at| program.body.get(at)) else {
        return false;
    };
    let mut scan = StrictAt {
        target: statement,
        strict: top.strict,
        found: None,
    };
    scan.visit_statement(candidate);
    scan.found.unwrap_or(false)
}

pub fn context_is_strict(program: &Program<'_>, statement: Span) -> bool {
    let mut scan = StrictAt {
        target: statement,
        strict: program_is_strict(program),
        found: None,
    };
    scan.visit_program(program);
    scan.found.unwrap_or(false)
}

pub fn names_stamped_array(scoping: &Scoping) -> bool {
    scoping
        .symbol_names()
        .any(|name| may_name_machinery(name) && string_array_unit(name).is_some())
}

pub fn has_stamped_array(program: &Program<'_>) -> bool {
    let mut probe = ArrayProbe::default();
    probe.visit_program(program);
    probe.found
}

pub fn apply_stamps<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    renames: &Renames<'a>,
    stamped_now: Vec<(usize, Span)>,
) {
    if renames.is_empty() {
        return;
    }
    let count = renames.apply(program, cx.scoping());
    cx.changed_by(count);
    for (length, statement) in stamped_now {
        cx.progress(notes::string_array_found(length), Some(statement));
    }
}

fn within(scoping: &Scoping, scope: ScopeId, ancestor: ScopeId) -> bool {
    scope == ancestor || scoping.scope_is_descendant_of(scope, ancestor)
}

fn spelled_outside(scoping: &Scoping, block: ScopeId, name: &str) -> bool {
    let home = scoping.scope_ancestors(block).find(|scope| {
        let flags = scoping.scope_flags(*scope);
        flags.is_top() || flags.is_function()
    });
    let Some(home) = home else {
        return true;
    };
    scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_name(*symbol) == name)
        .flat_map(|symbol| scoping.get_resolved_reference_ids(symbol).iter())
        .map(|reference| scoping.get_reference(*reference).scope_id())
        .any(|scope| within(scoping, scope, home) && !within(scoping, scope, block))
}

pub fn reached_outside_its_block(scoping: &Scoping, symbol: SymbolId) -> Option<&'static str> {
    let block = scoping.symbol_scope_id(symbol);
    let flags = scoping.scope_flags(block);
    if flags.is_top() || flags.is_function() {
        return None;
    }
    let name = scoping.symbol_name(symbol);
    let reached = scoping.root_unresolved_references().contains_key(name)
        || spelled_outside(scoping, block, name);
    reached.then_some(notes::MACHINERY_IN_A_BLOCK)
}

pub struct DynamicScopes {
    withs: Vec<ScopeId>,
}

impl DynamicScopes {
    pub fn of(scoping: &Scoping) -> Self {
        if !untwist_core::semantic::with_observes() {
            return Self { withs: Vec::new() };
        }
        let withs = scoping
            .scope_descendants_from_root()
            .filter(|id| scoping.scope_flags(*id).contains(ScopeFlags::With))
            .collect();
        Self { withs }
    }

    pub fn refusal(&self, scoping: &Scoping, symbol: SymbolId) -> Option<&'static str> {
        let scope = scoping.symbol_scope_id(symbol);
        if scoping.scope_flags(scope).contains_direct_eval() {
            return Some(notes::DIRECT_EVAL_SCOPE);
        }
        self.sees_with(scoping, symbol)
            .then_some(notes::WITH_STATEMENT)
    }

    pub fn sees_with(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        let scope = scoping.symbol_scope_id(symbol);
        let related = |with: &ScopeId| {
            *with == scope
                || scoping.scope_is_descendant_of(*with, scope)
                || scoping.scope_is_descendant_of(scope, *with)
        };
        self.withs.iter().any(related)
    }
}
