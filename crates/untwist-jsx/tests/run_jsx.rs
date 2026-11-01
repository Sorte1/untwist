use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_jsx::{run_jsx, JSX_ORDER};

fn run(source: &str, unsafe_transforms: bool) -> (String, Vec<&'static str>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        unsafe_transforms,
        ..PassOptions::default()
    };
    let (_sem, report) = run_jsx(&alloc, &mut parsed.program, sem, &opts);
    let names = report.per_pass.iter().map(|(name, _)| *name).collect();
    (print(&parsed.program, PrintStyle::Pretty), names)
}

#[test]
fn run_jsx_runs_both_runtimes_in_jsx_order() {
    let (code, names) = run(
        "React.createElement(\"div\", null);\njsx(\"span\", {});",
        true,
    );
    assert_eq!(code, "<div />;\n<span />;");
    assert_eq!(names, JSX_ORDER);
}

#[test]
fn run_jsx_runs_nothing_without_the_unsafe_flag() {
    let source = "React.createElement(\"div\", null);";
    let (code, names) = run(source, false);
    assert_eq!(code, source);
    assert!(names.is_empty(), "{names:?}");
}
