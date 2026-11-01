use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::collect::collect;
use untwist_rename::generate::{sequential_name, uid_candidates};
use untwist_rename::holders::ScopeTree;
use untwist_rename::oracle::{NameOracle, OuterRule};
use untwist_rename::reserved::Excluded;
use untwist_testing::{js_files_in, real_world_fixture, Allocator};

const CORPORA: [&str; 4] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/corpus-npm",
];
const SEQUENCE_PROBES: usize = 64;
const UID_PROBES: usize = 16;
const SHUFFLE_SEED: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Copy, Debug)]
enum Oracle {
    Plain,
    Keeping,
}

fn candidates(current: &str) -> Vec<String> {
    (0..SEQUENCE_PROBES)
        .map(sequential_name)
        .chain(uid_candidates(current).take(UID_PROBES))
        .collect()
}

fn shuffled(mut symbols: Vec<SymbolId>) -> Vec<SymbolId> {
    let mut state = SHUFFLE_SEED;
    for index in (1..symbols.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let pick = (state >> 33) as usize % (index + 1);
        symbols.swap(index, pick);
    }
    symbols
}

fn check_every_candidate(oracle: &mut NameOracle<'_>, symbols: Vec<SymbolId>, what: &str) -> usize {
    let mut checked = 0;
    for symbol in symbols {
        let current = oracle.current_name(symbol).to_string();
        let mut chosen = None;
        for candidate in candidates(&current) {
            let indexed = oracle.is_available(&candidate, symbol);
            let climbed = oracle.is_available_by_climb_for_tests(&candidate, symbol);
            assert_eq!(indexed, climbed, "{what}: {candidate} for {current}");
            checked += 1;
            if indexed {
                chosen = Some(candidate);
                break;
            }
        }
        if let Some(name) = chosen {
            oracle.take(symbol, std::iter::once(name));
        }
    }
    checked
}

fn every_order_agrees(name: &str, source: &str) -> usize {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    assert!(
        ScopeTree::new(&sem.scoping).is_numbered(),
        "{name}: the scope ids are not a preorder, so the index is never consulted"
    );
    agrees_on(name, &parsed.program, &sem.scoping)
}

fn agrees_on(name: &str, program: &Program<'_>, scoping: &Scoping) -> usize {
    let mut checked = 0;
    for rule in [OuterRule::UsedEnclosing, OuterRule::ProgramWide] {
        let collected = collect(program, scoping, rule);
        let symbols: Vec<SymbolId> = collected.bindings.iter().map(|info| info.symbol).collect();
        for kind in [Oracle::Plain, Oracle::Keeping] {
            for order in [symbols.clone(), shuffled(symbols.clone())] {
                let mut oracle = match kind {
                    Oracle::Plain => NameOracle::new(scoping, &collected),
                    Oracle::Keeping => {
                        NameOracle::keeping(scoping, &collected, Excluded::from_scoping(scoping))
                    }
                };
                oracle.defer(symbols.iter().copied());
                let what = format!("{name} {rule:?} {kind:?}");
                checked += check_every_candidate(&mut oracle, order, &what);
            }
        }
    }
    checked
}

#[test]
fn the_index_answers_every_candidate_as_the_climb_does_on_the_rename_corpora() {
    let mut checked = 0;
    for corpus in CORPORA {
        for (name, source) in js_files_in(corpus) {
            checked += every_order_agrees(&name, &source);
        }
    }
    assert!(checked > 10_000, "{checked} candidates checked");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-rename --test oracle_index"
)]
fn the_index_answers_every_candidate_as_the_climb_does_on_game_js() {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    assert!(every_order_agrees("game.js", &source) > 1_000_000);
}

const NESTED: &str = "function f(a) { { let b = a; } } function g(c) { let d = c; } var e = f;";

fn scope(index: usize) -> ScopeId {
    ScopeId::from_usize(index)
}

#[test]
fn each_scope_spans_its_preorder_interval() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, NESTED);
    let sem = build_scoping(&parsed.program);
    let tree = ScopeTree::new(&sem.scoping);
    assert!(tree.is_numbered());
    let spans: Vec<(Option<usize>, Option<usize>)> = (0..sem.scoping.scopes_len())
        .map(|index| (tree.order(scope(index)), tree.last(scope(index))))
        .collect();
    assert_eq!(
        spans,
        vec![
            (Some(0), Some(3)),
            (Some(1), Some(2)),
            (Some(2), Some(2)),
            (Some(3), Some(3)),
        ]
    );
    assert_eq!(tree.order(scope(4)), None);
}

#[test]
fn a_scope_tree_whose_ids_are_not_a_preorder_falls_back_to_the_climb() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, NESTED);
    let mut sem = build_scoping(&parsed.program);
    sem.scoping.set_scope_parent_id(scope(2), Some(scope(3)));
    let tree = ScopeTree::new(&sem.scoping);
    assert!(!tree.is_numbered());
    assert_eq!(tree.order(scope(0)), None);
    assert!(agrees_on("reparented", &parsed.program, &sem.scoping) > 0);
}

const CROWD: usize = 64;

fn crowded_before_an_ancestor() -> String {
    let mut source: String = (0..CROWD)
        .map(|index| format!("function f{index}() {{ var held = {index}; }}\n"))
        .collect();
    source.push_str(
        "function outer() { var held = 1; function inner() { var probe = 0; return probe; } return inner; }\n",
    );
    source
}

#[test]
fn an_ancestor_behind_a_long_run_of_earlier_holders_still_conflicts_when_keeping() {
    let source = crowded_before_an_ancestor();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let oracle = NameOracle::keeping(
        &sem.scoping,
        &collected,
        Excluded::from_scoping(&sem.scoping),
    );
    let probe = sem
        .scoping
        .symbol_ids()
        .find(|symbol| sem.scoping.symbol_name(*symbol) == "probe");
    let Some(probe) = probe else {
        panic!("no probe binding");
    };
    assert!(!oracle.is_available("held", probe));
    assert!(!oracle.is_available_by_climb_for_tests("held", probe));
    assert!(oracle.is_available("fresh", probe));
    assert!(agrees_on("crowded", &parsed.program, &sem.scoping) > 0);
}

const SHAPE_SCOPES: usize = 25;
const SHAPE_TREES: usize = 2_000;
const SHAPE_SEED: u64 = 0x2545_f491_4f6c_dd1d;

struct Seeded(u64);

impl Seeded {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize % bound
    }
}

fn random_preorder_parents(rng: &mut Seeded) -> Vec<Option<usize>> {
    let mut parents = vec![None];
    let mut open = vec![0];
    for index in 1..SHAPE_SCOPES {
        let keep = 1 + rng.below(open.len());
        open.truncate(keep);
        parents.push(open.last().copied());
        open.push(index);
    }
    parents
}

fn is_preorder(parents: &[Option<usize>]) -> bool {
    let is_ancestor = |ancestor: usize, mut scope: usize| {
        while let Some(Some(parent)) = parents.get(scope) {
            if *parent == ancestor {
                return true;
            }
            scope = *parent;
        }
        false
    };
    parents
        .iter()
        .enumerate()
        .all(|(index, parent)| match parent {
            None => index == 0,
            Some(parent) => {
                *parent < index && (parent + 1..index).all(|between| is_ancestor(*parent, between))
            }
        })
}

fn reshape(scoping: &mut Scoping, parents: &[Option<usize>]) {
    for (index, parent) in parents.iter().enumerate() {
        scoping.set_scope_parent_id(scope(index), parent.map(scope));
    }
}

fn intervals_match_the_climb(tree: &ScopeTree, scoping: &Scoping) -> bool {
    (0..SHAPE_SCOPES).all(|outer| {
        (0..SHAPE_SCOPES).all(|inner| {
            let (Some(first), Some(last), Some(order)) = (
                tree.order(scope(outer)),
                tree.last(scope(outer)),
                tree.order(scope(inner)),
            ) else {
                return false;
            };
            (first < order && order <= last)
                == scoping.scope_is_descendant_of(scope(inner), scope(outer))
        })
    })
}

#[test]
fn the_preorder_check_accepts_exactly_the_preorders_over_random_scope_trees() {
    let source: String = (1..SHAPE_SCOPES)
        .map(|index| format!("function f{index}() {{}}\n"))
        .collect();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let mut sem = build_scoping(&parsed.program);
    assert_eq!(sem.scoping.scopes_len(), SHAPE_SCOPES);
    let mut rng = Seeded(SHAPE_SEED);
    let (mut accepted, mut rejected) = (0, 0);
    for tree_index in 0..SHAPE_TREES {
        let mut parents = random_preorder_parents(&mut rng);
        assert!(is_preorder(&parents), "tree {tree_index}: {parents:?}");
        if tree_index % 2 == 1 {
            let moved = 1 + rng.below(SHAPE_SCOPES - 1);
            parents[moved] = Some(rng.below(SHAPE_SCOPES));
        }
        reshape(&mut sem.scoping, &parents);
        let tree = ScopeTree::new(&sem.scoping);
        let expected = is_preorder(&parents);
        assert_eq!(
            tree.is_numbered(),
            expected,
            "tree {tree_index}: {parents:?}"
        );
        if expected {
            assert!(
                intervals_match_the_climb(&tree, &sem.scoping),
                "tree {tree_index}: {parents:?}"
            );
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(accepted > SHAPE_TREES / 2, "{accepted} accepted");
    assert!(rejected > SHAPE_TREES / 4, "{rejected} rejected");
}
