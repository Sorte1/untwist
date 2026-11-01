use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::readonly::{
    readonly_binding_uses, readonly_bindings_uses, scan_object_uses, scan_symbol_uses,
};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_testing::js_files_in;

#[derive(Default)]
struct LiteralInitialisers {
    symbols: Vec<SymbolId>,
    widest: Option<(usize, SymbolId)>,
}

impl<'a> Visit<'a> for LiteralInitialisers {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let width = match &it.init {
            Some(Expression::ArrayExpression(array)) => Some(array.elements.len()),
            Some(Expression::ObjectExpression(object)) => Some(object.properties.len()),
            _ => None,
        };
        if let (Some(width), BindingPattern::BindingIdentifier(id)) = (width, &it.id) {
            if let Some(symbol) = id.symbol_id.get() {
                self.symbols.push(symbol);
                if self.widest.is_none_or(|(seen, _)| width > seen) {
                    self.widest = Some((width, symbol));
                }
            }
        }
        walk::walk_variable_declarator(self, it);
    }
}

fn assert_batched_matches_single(label: &str, source: &str, sample: impl Fn(usize) -> bool) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{label}: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let mut found = LiteralInitialisers::default();
    found.visit_program(&parsed.program);
    let all: FxHashSet<SymbolId> = sem.scoping.symbol_ids().collect();
    let batched = scan_symbol_uses(&parsed.program, &sem.scoping, &all);
    let readonly = readonly_bindings_uses(&parsed.program, &sem.scoping, &all);
    let mut checked = 0usize;
    for (index, symbol) in sem.scoping.symbol_ids().enumerate() {
        let is_widest = found.widest.is_some_and(|(_, widest)| widest == symbol);
        if !sample(index) && !is_widest {
            continue;
        }
        checked += 1;
        let single = scan_object_uses(&parsed.program, &sem.scoping, symbol);
        assert_eq!(
            batched.of(symbol),
            Some(&single),
            "{label}: {} differs batched vs single",
            sem.scoping.symbol_name(symbol)
        );
        assert_eq!(
            readonly.get(&symbol),
            readonly_binding_uses(&parsed.program, &sem.scoping, symbol).as_ref(),
            "{label}: {} differs in the readonly map",
            sem.scoping.symbol_name(symbol)
        );
    }
    assert!(checked > 0, "{label}: nothing was compared");
}

#[test]
fn every_deobfuscate_fixture_scans_the_same_batched_and_single() {
    let fixtures = js_files_in("fixtures/reference/deobfuscate");
    assert!(fixtures.len() >= 15, "{}", fixtures.len());
    for (label, source) in fixtures {
        assert_batched_matches_single(&label, &source, |_| true);
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-deobfuscate --test readonly_batched"
)]
fn game_js_scans_the_same_batched_and_single_on_its_literal_declarators() {
    let Some(source) = untwist_testing::real_world_fixture("game.js") else {
        return;
    };
    assert_batched_matches_single("game.js", &source, |index| index % 200 == 0);
}
