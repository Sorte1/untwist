use std::sync::{Mutex, MutexGuard, PoisonError};

use oxc_allocator::Allocator;
use untwist::{Options, Output, Pipeline, StopReason};
use untwist_core::parse::parse_program;
use untwist_core::pass::{
    cascade_off_for_tests, full_rounds_for_tests, probe_variants_for_tests, run_to_fixpoint,
    run_to_fixpoint_noting_last, skip_innermost_list_for_tests, stale_settled_for_tests,
    take_variant_violations_for_tests, treat_as_neutral_for_tests, walk_every_group_for_tests,
    walk_every_list_for_tests, Cx, Interest, Locality, Note, Pass, PassGroup, PassOptions,
};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::run_cleanup;
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, Copy)]
struct Mode {
    name: &'static str,
    unsafe_transforms: bool,
    extended: bool,
}

const MODES: [Mode; 4] = [
    Mode {
        name: "default",
        unsafe_transforms: false,
        extended: true,
    },
    Mode {
        name: "conservative",
        unsafe_transforms: false,
        extended: false,
    },
    Mode {
        name: "unsafe",
        unsafe_transforms: true,
        extended: true,
    },
    Mode {
        name: "unsafe conservative",
        unsafe_transforms: true,
        extended: false,
    },
];

fn options(mode: Mode) -> Options {
    Options {
        unsafe_transforms: mode.unsafe_transforms,
        extended: mode.extended,
        ..Options::default()
    }
}

type StageRow = (
    &'static str,
    u32,
    u32,
    u32,
    Vec<(&'static str, u32)>,
    Vec<(&'static str, u32)>,
    StopReason,
);

#[derive(Debug, PartialEq)]
struct Observed {
    codes: Vec<String>,
    notes: Vec<Note>,
    stages: Vec<StageRow>,
    panicked: Vec<usize>,
    depth_refused: Vec<usize>,
}

fn observed(out: Output) -> Observed {
    let stages = out
        .stages
        .iter()
        .map(|stage| {
            (
                stage.stage,
                stage.round,
                stage.iterations,
                stage.changes,
                stage.per_pass.clone(),
                stage.fixpoint_per_pass.clone(),
                stage.stop,
            )
        })
        .collect();
    Observed {
        codes: out.codes,
        notes: out.notes,
        stages,
        panicked: out.panicked,
        depth_refused: out.depth_refused,
    }
}

fn run(source: &str, mode: Mode, full: bool) -> Observed {
    full_rounds_for_tests(full);
    let out = Pipeline::new(options(mode)).run(source);
    full_rounds_for_tests(false);
    observed(out)
}

fn confirming_changes(out: &Output) -> u32 {
    out.stages
        .iter()
        .flat_map(|row| row.work.iter())
        .filter(|traversal| traversal.confirming)
        .map(|traversal| traversal.changes)
        .sum()
}

fn assert_scoped_equals_full(label: &str, source: &str) {
    for mode in MODES {
        let scoped = run(source, mode, false);
        let full = run(source, mode, true);
        assert!(
            scoped == full,
            "{label} ({}): the scoped rounds diverge from forced full rounds",
            mode.name
        );
    }
}

const DIFF_SETS: [&str; 6] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/behaviour",
    "fixtures/corpus-npm",
];

const UNTRANSPILE_SETS: [&str; 7] = [
    "fixtures/untranspile/babel",
    "fixtures/untranspile/esbuild",
    "fixtures/untranspile/hand",
    "fixtures/untranspile/samples",
    "fixtures/untranspile/swc",
    "fixtures/untranspile/terser",
    "fixtures/untranspile/ts",
];

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

fn corpora(slow: bool) -> Vec<(String, String)> {
    DIFF_SETS
        .iter()
        .chain(UNTRANSPILE_SETS.iter())
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

#[test]
fn the_corpora_equal_forced_full_rounds() {
    let _serial = serial();
    let files = corpora(false);
    assert!(files.len() >= 400, "{}", files.len());
    for (label, source) in files {
        assert_scoped_equals_full(&label, &source);
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test scoped_rounds"
)]
fn game_js_and_the_high_preset_equal_forced_full_rounds() {
    let _serial = serial();
    for (label, source) in slow_files() {
        assert_scoped_equals_full(&label, &source);
        assert_parked_equals_walked(&label, &source);
        assert_steered_equals_every_list(&label, &source);
    }
    if let Some(source) = real_world_fixture("game.js") {
        cascade_off_for_tests(true);
        assert!(steered_away(&source, "unminify") > 0);
        skip_innermost_list_for_tests(true);
        let mutant = run(&source, MODES[0], false);
        skip_innermost_list_for_tests(false);
        let full = run(&source, MODES[0], true);
        cascade_off_for_tests(false);
        assert!(mutant != full);
    }
}

fn slow_files() -> Vec<(String, String)> {
    let mut files = corpora(true);
    assert_eq!(files.len(), 2);
    files.extend(real_world_fixture("game.js").map(|source| ("game.js".to_string(), source)));
    files
}

fn assert_parked_equals_walked(label: &str, source: &str) {
    for mode in MODES {
        let parked = run(source, mode, false);
        walk_every_group_for_tests(true);
        let walked = run(source, mode, false);
        walk_every_group_for_tests(false);
        assert!(
            parked == walked,
            "{label} ({}): parking a program-only group's body diverges from walking it",
            mode.name
        );
    }
}

fn every_list(source: &str, mode: Mode) -> Observed {
    walk_every_list_for_tests(true);
    let walked = run(source, mode, false);
    walk_every_list_for_tests(false);
    walked
}

fn assert_steered_equals_every_list(label: &str, source: &str) {
    for mode in MODES {
        let out = Pipeline::new(options(mode)).run(source);
        assert_eq!(
            confirming_changes(&out),
            0,
            "{label} ({}): a confirming full round changed the program",
            mode.name
        );
        let steered = observed(out);
        let walked = every_list(source, mode);
        assert!(
            steered == walked,
            "{label} ({}): parking clean statement lists diverges from walking every list",
            mode.name
        );
    }
}

#[test]
fn l1_the_corpora_equal_runs_that_walk_every_list() {
    let _serial = serial();
    for (label, source) in corpora(false) {
        assert_steered_equals_every_list(&label, &source);
    }
}

fn steered_away(source: &str, stage: &str) -> u32 {
    let out = Pipeline::new(options(MODES[0])).run(source);
    out.stages
        .iter()
        .filter(|row| row.stage == stage)
        .flat_map(|row| row.work.iter())
        .map(|traversal| traversal.lists_parked + traversal.statements_shelved)
        .sum()
}

struct Peel;

impl<'a> Pass<'a> for Peel {
    fn name(&self) -> &'static str {
        "peel"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT
    }

    fn exit_statement(&mut self, stmt: &mut oxc_ast::ast::Statement<'a>, cx: &mut Cx<'a, '_>) {
        use oxc_ast::ast::{Expression, Statement, UnaryOperator};
        let Statement::ExpressionStatement(statement) = stmt else {
            return;
        };
        let Expression::UnaryExpression(outer) = &mut statement.expression else {
            return;
        };
        if outer.operator != UnaryOperator::LogicalNot {
            return;
        }
        let Expression::UnaryExpression(inner) = &mut outer.argument else {
            return;
        };
        if inner.operator != UnaryOperator::LogicalNot {
            return;
        }
        let span = inner.span;
        let placeholder = Expression::new_null_literal(span, &*cx);
        let argument = std::mem::replace(&mut inner.argument, placeholder);
        statement.expression = argument;
        cx.changed();
    }
}

struct Notices;

impl<'a> Pass<'a> for Notices {
    fn name(&self) -> &'static str {
        "notices"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT
    }

    fn exit_statement(&mut self, stmt: &mut oxc_ast::ast::Statement<'a>, cx: &mut Cx<'a, '_>) {
        if let oxc_ast::ast::Statement::DebuggerStatement(statement) = stmt {
            cx.note("debugger kept", Some(statement.span));
        }
    }
}

const NOTED_CASCADE: &str = "function f() {\n  debugger;\n}\nfunction g() {\n  h();\n}\nfunction i() {\n  j();\n}\n!!!!!!!!k;\n";

fn noted(full: bool) -> (Option<Vec<Note>>, Vec<Note>, u32) {
    full_rounds_for_tests(full);
    untwist_core::pass::take_work_for_tests();
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, NOTED_CASCADE);
    let sem = build_scoping(&parsed.program);
    let passes: Vec<Box<dyn Pass<'_>>> = vec![Box::new(Peel), Box::new(Notices)];
    let mut group = PassGroup::new(passes, PassOptions::default());
    let (_, report, last) =
        run_to_fixpoint_noting_last(&alloc, &mut parsed.program, sem, &mut group);
    full_rounds_for_tests(false);
    let shelved = untwist_core::pass::take_work_for_tests()
        .iter()
        .map(|traversal| traversal.lists_parked + traversal.statements_shelved)
        .sum();
    (last, report.notes, shelved)
}

#[test]
fn n1_a_note_from_a_parked_region_is_carried_into_every_steered_round() {
    let _serial = serial();
    let (full_last, full_notes, _) = noted(true);
    let (last, notes, shelved) = noted(false);
    assert!(shelved > 0);
    assert_eq!(last, full_last);
    assert_eq!(notes, full_notes);
    assert_eq!(notes.len(), 1, "{notes:#?}");
}

#[derive(Default)]
struct Census {
    seen: usize,
}

const CENSUS: &str = "census";

impl<'a> Pass<'a> for Census {
    fn name(&self) -> &'static str {
        CENSUS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_PROGRAM | Interest::EXIT_STATEMENT
    }

    fn enter_program(&mut self, _program: &mut oxc_ast::ast::Program<'a>, _cx: &mut Cx<'a, '_>) {
        self.seen = 0;
    }

    fn exit_statement(&mut self, _stmt: &mut oxc_ast::ast::Statement<'a>, _cx: &mut Cx<'a, '_>) {
        self.seen += 1;
    }

    fn exit_program(&mut self, _program: &mut oxc_ast::ast::Program<'a>, cx: &mut Cx<'a, '_>) {
        cx.progress(format!("census saw {} statements", self.seen), None);
    }
}

const TWO_CASCADES: &str = "function f() {\n  function g() {\n    !!!!!!!!!!a;\n  }\n  !!!!!!!!b;\n}\nfunction h() {}\nfunction i() {}\nfunction j() {}\n";

type Peeled = (String, u32, Vec<(&'static str, u32)>);

fn peeled(source: &str, full: bool) -> Peeled {
    full_rounds_for_tests(full);
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let passes: Vec<Box<dyn Pass<'_>>> = vec![Box::new(Peel)];
    let mut group = PassGroup::new(passes, PassOptions::default());
    let (_, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    full_rounds_for_tests(false);
    let printed = print(&parsed.program, PrintStyle::Compact);
    (printed, report.iterations, report.per_pass)
}

#[test]
fn c4_a_parked_ancestor_of_an_edit_diverges_on_a_named_source() {
    let _serial = serial();
    let full = peeled(TWO_CASCADES, true);
    assert_eq!(peeled(TWO_CASCADES, false), full);
    skip_innermost_list_for_tests(true);
    let mutant = peeled(TWO_CASCADES, false);
    skip_innermost_list_for_tests(false);
    assert_ne!(mutant, full);
}

const CENSUS_SOURCE: &str =
    "function a() {\n  b();\n  c();\n}\nfunction d() {\n  e();\n}\n!!!!!!f;\n";

fn census(full: bool) -> (Vec<Note>, String) {
    full_rounds_for_tests(full);
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CENSUS_SOURCE);
    let sem = build_scoping(&parsed.program);
    let passes: Vec<Box<dyn Pass<'_>>> = vec![Box::new(Peel), Box::new(Census::default())];
    let mut group = PassGroup::new(passes, PassOptions::default());
    let (_, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    full_rounds_for_tests(false);
    (report.notes, print(&parsed.program, PrintStyle::Compact))
}

#[test]
fn c5_a_program_pass_taken_as_neutral_diverges_on_a_named_source() {
    let _serial = serial();
    let full = census(true);
    assert_eq!(census(false), full);
    treat_as_neutral_for_tests(Some(CENSUS));
    let mutant = census(false);
    treat_as_neutral_for_tests(None);
    assert_ne!(mutant, full);
    assert_eq!(mutant.1, full.1);
}

#[test]
fn w1_the_corpora_equal_runs_that_walk_every_group() {
    let _serial = serial();
    for (label, source) in corpora(false) {
        assert_parked_equals_walked(&label, &source);
    }
}

struct Reports(std::rc::Rc<std::cell::Cell<(usize, usize)>>);

impl<'a> Pass<'a> for Reports {
    fn name(&self) -> &'static str {
        "sees-the-body"
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_PROGRAM
    }

    fn enter_program(&mut self, program: &mut oxc_ast::ast::Program<'a>, _cx: &mut Cx<'a, '_>) {
        let (_, exited) = self.0.get();
        self.0.set((program.body.len(), exited));
    }

    fn exit_program(&mut self, program: &mut oxc_ast::ast::Program<'a>, _cx: &mut Cx<'a, '_>) {
        let (entered, _) = self.0.get();
        self.0.set((entered, program.body.len()));
    }
}

#[test]
fn w2_a_program_only_group_hands_its_exit_hooks_the_whole_body() {
    let _serial = serial();
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "a();\nb();\nc();\n");
    let sem = build_scoping(&parsed.program);
    let seen = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
    let pass: Box<dyn Pass<'_>> = Box::new(Reports(std::rc::Rc::clone(&seen)));
    let mut group = PassGroup::new(vec![pass], PassOptions::default());
    run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    assert_eq!(seen.get(), (3, 3));
    assert_eq!(parsed.program.body.len(), 3);
}

const SETTLE_CHAIN: &str =
    "function f() {\n  var o = { k: \"YW\" };\n  return atob(o.k + \"Jj\");\n}\nconsole.log(f());\n";

#[test]
fn a_settle_chain_equals_forced_full_rounds() {
    let _serial = serial();
    assert_scoped_equals_full("the settle chain", SETTLE_CHAIN);
    let settled = run(SETTLE_CHAIN, MODES[0], false);
    let cleanups = settled
        .stages
        .iter()
        .filter(|stage| stage.0 == "cleanup")
        .count();
    assert_eq!(cleanups, 3, "{:#?}", settled.stages);
    assert!(!settled.codes.join("\n").contains("atob"));
}

const STALE_REPLAY_FIXTURE: &str =
    "2.15.6--rc4-rot-noshuf-noshift-istr-w3f-t1-split--iife--terser--functions-1.js";

#[test]
fn a_settled_replay_that_ignores_edits_diverges_on_a_named_fixture() {
    let _serial = serial();
    let source = js_files_in("fixtures/obfuscator-versions")
        .into_iter()
        .find(|(label, _)| label.ends_with(STALE_REPLAY_FIXTURE))
        .map(|(_, source)| source)
        .expect(STALE_REPLAY_FIXTURE);
    stale_settled_for_tests(true);
    let stale = run(&source, MODES[2], false);
    stale_settled_for_tests(false);
    let settled = run(&source, MODES[2], false);
    let full = run(&source, MODES[2], true);
    assert!(stale != full);
    assert!(settled == full);
    assert!(replays(&source, MODES[2], false) > 0);
    assert_eq!(replays(&source, MODES[2], true), 0);
}

fn replays(source: &str, mode: Mode, full: bool) -> usize {
    full_rounds_for_tests(full);
    let out = Pipeline::new(options(mode)).run(source);
    full_rounds_for_tests(false);
    out.stages
        .iter()
        .filter(|stage| stage.stage == "deobfuscate")
        .flat_map(|stage| stage.iteration_elapsed.iter())
        .filter(|timing| timing.traverse().is_zero())
        .count()
}

struct Acted {
    label: &'static str,
    source: &'static str,
    mode: Mode,
    unminify: bool,
    absent: &'static str,
}

const CFO_CALL: &str =
    "console.log({ \"abc\": function (a, b) { return a + b; }, \"x\": \"y\" }[\"abc\"](1, 2));\n";

const CFO_COMPUTED: &str =
    "console.log({ \"def\": \"text\", \"abc\": function (a, b) { return a + b; } }[\"def\"]);\n";

const CFO_STATIC: &str =
    "console.log({ \"def\": \"text\", \"abc\": function (a, b) { return a + b; } }.def);\n";

const DEOBFUSCATE_HOOKS: [Acted; 8] = [
    Acted {
        label: "inline-object-props on a computed member of a literal",
        source: "var r = { a: 1, bc: 2 }[\"bc\"];\nconsole.log(r);\n",
        mode: MODES[0],
        unminify: false,
        absent: "bc",
    },
    Acted {
        label: "inline-object-props on a static member of a literal",
        source: "var s = { a: 1, b: 2 }.b;\nconsole.log(s);\n",
        mode: MODES[0],
        unminify: true,
        absent: "b: 2",
    },
    Acted {
        label: "dead-code on an opaque conditional",
        source: "var x = \"ab\" === \"ab\" ? f() : g();\nconsole.log(x);\n",
        mode: MODES[2],
        unminify: true,
        absent: "g()",
    },
    Acted {
        label: "evaluate-globals on a call",
        source: "var y = decodeURIComponent(\"%41%42\");\nconsole.log(y);\n",
        mode: MODES[0],
        unminify: true,
        absent: "decodeURIComponent",
    },
    Acted {
        label: "fold-literal-chains on a call",
        source: "var z = \"abc\".split(\"\").reverse().join(\"\");\nconsole.log(z);\n",
        mode: MODES[0],
        unminify: true,
        absent: "reverse",
    },
    Acted {
        label: "control-flow-object on a call",
        source: CFO_CALL,
        mode: MODES[0],
        unminify: false,
        absent: "function",
    },
    Acted {
        label: "control-flow-object on a computed member",
        source: CFO_COMPUTED,
        mode: MODES[0],
        unminify: false,
        absent: "function",
    },
    Acted {
        label: "control-flow-object on a static member",
        source: CFO_STATIC,
        mode: MODES[0],
        unminify: false,
        absent: "function",
    },
];

#[test]
fn every_deobfuscate_variant_set_covers_the_variants_its_hooks_act_on() {
    let _serial = serial();
    for case in DEOBFUSCATE_HOOKS {
        take_variant_violations_for_tests();
        probe_variants_for_tests(true);
        let probed = Pipeline::new(Options {
            unminify: case.unminify,
            ..options(case.mode)
        })
        .run(case.source);
        probe_variants_for_tests(false);
        let violations = take_variant_violations_for_tests();
        assert!(
            violations.is_empty(),
            "{}: a pass acted outside its declared variants: {violations:#?}",
            case.label
        );
        let code = probed.codes.join("\n");
        assert!(
            !code.contains(case.absent),
            "{}: the hook did not act: {code}",
            case.label
        );
    }
}

#[test]
fn a_reused_pass_options_never_replays_cleanup_on_another_program() {
    let _serial = serial();
    let alloc = Allocator::default();
    let opts = untwist_core::pass::PassOptions::default();
    let mut parsed = parse_program(
        &alloc,
        "console.log(\"xy\".split(\"\").reverse().join(\"\"));",
    );
    let sem = build_scoping(&parsed.program);
    let (_, first) = run_cleanup(&alloc, &mut parsed.program, sem, &opts);
    assert!(first.total_changes > 0);
    parsed = parse_program(
        &alloc,
        "console.log(\"abc\".split(\"\").reverse().join(\"\"));",
    );
    let sem = build_scoping(&parsed.program);
    let (_, second) = run_cleanup(&alloc, &mut parsed.program, sem, &opts);
    let printed = print(&parsed.program, PrintStyle::Compact);
    assert!(
        printed.contains("cba") && !printed.contains("split"),
        "{printed}"
    );
    assert!(second.total_changes > 0);
}
