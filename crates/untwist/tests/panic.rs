#[path = "common/panic_on_marker.rs"]
mod panic_on_marker;

use panic_on_marker::panicking;
use untwist::{Options, Pipeline, Plugins, Stage};

fn options() -> Options {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterParse, panicking as untwist::PluginCtor);
    Options {
        plugins,
        unpack: false,
        ..Options::default()
    }
}

#[test]
fn a_panicking_pass_does_not_take_down_the_batch() {
    let out = Pipeline::new(options()).run_many(&["f(\"ok\");", "g(\"boom\");", "h(\"fine\");"]);
    assert_eq!(out.codes.len(), 3);
    assert_eq!(out.codes[0], "f(\"ok\");");
    assert_eq!(out.codes[2], "h(\"fine\");");
    assert_eq!(out.panicked, vec![1]);
    assert!(
        out.notes
            .iter()
            .any(|note| note.message == untwist::PANICKED),
        "{:?}",
        out.notes
    );
}

#[test]
fn a_clean_batch_reports_no_panic() {
    let out = Pipeline::new(options()).run_many(&["f(\"ok\");", "h(\"fine\");"]);
    assert!(out.panicked.is_empty());
    assert!(!out
        .notes
        .iter()
        .any(|note| note.message == untwist::PANICKED));
}

#[test]
fn the_panicking_input_still_produces_code() {
    let out = Pipeline::new(options()).run("g(\"boom\");");
    assert_eq!(out.panicked, vec![0]);
    assert_eq!(out.codes.len(), 1);
    assert!(out.codes[0].contains("boom"));
}

#[test]
fn the_panicking_input_prints_the_tree_as_far_as_the_passes_got() {
    let out = Pipeline::new(options()).run("g(\"boom\"); var a = !0;");
    assert_eq!(out.panicked, vec![0]);
    assert_eq!(out.codes[0], "g(\"boom\");\nvar a = !0;");
}
