use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::eval_script_in;
use untwist_eval::test_hooks::{compile_census, frozen, record_units, UnitRecord};

fn both(source: &str) -> (UnitRecord, u64) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics.first()
    );
    let program = &parsed.program;
    let run = || eval_script_in(program, Budget::new(5_000_000), |_, _| ());
    let mut compiled = Vec::new();
    let census = compile_census(|| compiled = record_units(run));
    frozen("compile_deep", source, &compiled);
    (compiled.remove(0), census.calls)
}

fn on_a_2_mib_thread(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the row's thread finished");
}

#[test]
fn a_call_chain_past_the_nesting_cap_bails_where_the_walker_bails() {
    on_a_2_mib_thread(|| {
        let source = format!("var g = () => g; (() => g{})();", "()".repeat(200_000));
        let (record, calls) = both(&source);
        assert_eq!(record.bail.as_deref(), Some("nesting depth cap exceeded"));
        assert!(calls > 0);
    });
}

#[test]
fn a_99990_term_sum_runs_and_drops_compiled() {
    on_a_2_mib_thread(|| {
        let source = format!("var x = 1; (() => x{})();", " + x".repeat(99_989));
        let (record, calls) = both(&source);
        assert_eq!(record.values, ["number:40f8696000000000"]);
        assert_eq!(calls, 1);
    });
}

#[test]
fn a_2000_link_member_chain_compiles_and_drops() {
    on_a_2_mib_thread(|| {
        let source = format!(
            "var o = {{}}; o.a = o; o.f = () => 7; (() => o{}.f())();",
            ".a".repeat(2_000)
        );
        let (record, calls) = both(&source);
        assert_eq!(record.values, ["number:401c000000000000"]);
        assert_eq!(calls, 2);
    });
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the walker's own chain links overflow a debug 2 MiB thread by 3,000"
)]
fn a_15000_link_member_chain_runs_compiled_as_far_as_the_walker() {
    on_a_2_mib_thread(|| {
        let source = format!(
            "var o = {{}}; o.a = o; o.f = () => 7; (() => o{}.f())();",
            ".a".repeat(15_000)
        );
        let (record, calls) = both(&source);
        assert_eq!(record.values, ["number:401c000000000000"]);
        assert_eq!(calls, 2);
    });
}
