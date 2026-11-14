use oxc_span::Span;
use untwist_rename::modern::plan::{Installer, Keep};
use untwist_rename::modern::report::{
    detail, installer_note, is_detail, rewrite_note, summary, Status, Totals,
};

fn installer(module: &str, kept: Option<Keep>) -> Installer {
    Installer {
        modules: vec![format!("modules/{module}")],
        range: "3.39".to_string(),
        span: Span::new(0, 1),
        kept,
        with_others: false,
    }
}

#[test]
fn the_summary_counts_kept_modules_by_reason() {
    let removed = [installer("es.array.flat", None)];
    let kept = [
        installer("es.string.fixed", Some(Keep::Foreign)),
        installer("es.string.fontsize", Some(Keep::Foreign)),
        installer("es.typed-array.sort", Some(Keep::NotRun)),
    ];
    let totals = Totals {
        removed_helpers: 2,
        kept_helpers: 5,
        rewritten: 7,
    };
    let text = summary(
        &removed.iter().collect::<Vec<_>>(),
        &kept.iter().collect::<Vec<_>>(),
        &totals,
    );
    assert_eq!(
        text,
        "removed 1 of 4 core-js polyfill modules and 2 helper modules, kept 3 (2 need code not recognised as core-js, 1 not run as a statement); rewrote 7 helper calls to the native form; 5 helper modules are still used"
    );
}

#[test]
fn a_summary_with_nothing_kept_has_no_kept_part() {
    let removed = [installer("es.array.flat", None)];
    let totals = Totals {
        removed_helpers: 0,
        kept_helpers: 0,
        rewritten: 0,
    };
    let text = summary(&removed.iter().collect::<Vec<_>>(), &[], &totals);
    assert_eq!(
        text,
        "removed 1 of 1 core-js polyfill modules and 0 helper modules; 0 helper modules are still used"
    );
}

#[test]
fn every_per_module_line_reads_back_as_a_detail() {
    let kept = installer_note(&installer("es.string.fixed", Some(Keep::Foreign)));
    assert!(is_detail(&kept));
    let read = detail(&kept).unwrap();
    assert_eq!(read.status, Status::Kept);
    assert_eq!(
        read.reason,
        "it calls code that is not recognised as core-js"
    );
    assert_eq!(read.item, "core-js 3.39: modules/es.string.fixed.js");

    let removed = installer_note(&installer("es.array.flat", None));
    let read = detail(&removed).unwrap();
    assert_eq!(read.status, Status::Removed);
    assert_eq!(read.reason, "native in the baseline");
    assert_eq!(read.item, "core-js 3.39: modules/es.array.flat.js");

    let rewrote = rewrite_note("hasOwn(o, k) -> Object.hasOwn(o, k)", 70);
    let read = detail(&rewrote).unwrap();
    assert_eq!(read.status, Status::Rewrote);
    assert_eq!(read.item, "70 \u{d7} hasOwn(o, k) -> Object.hasOwn(o, k)");
}

#[test]
fn the_summary_and_assumption_lines_are_not_details() {
    assert!(!is_detail(
        "removed 1 of 4 core-js polyfill modules and 2 helper modules"
    ));
    assert!(!is_detail(
        "the output assumes an engine at least as new as chrome 120"
    ));
    assert!(detail("removed 1 of 4 core-js polyfill modules").is_none());
}
