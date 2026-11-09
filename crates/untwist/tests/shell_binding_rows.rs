use untwist::{Options, Pipeline, ANONYMOUS_SHELL_SCOPED};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/shell_binding_rows.txt");

fn rows(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
        .map(|(label, shell)| (label.to_string(), format!("{shell}()")))
        .collect()
}

fn options(deobfuscate: bool, unsafe_transforms: bool) -> Options {
    Options {
        unpack: false,
        jsx: false,
        deobfuscate,
        unsafe_transforms,
        ..Options::default()
    }
}

fn shell_of(called: &str) -> &str {
    called.strip_suffix("()").unwrap_or(called)
}

fn run(options: Options, called: &str) -> String {
    Pipeline::new(options)
        .run(shell_of(called))
        .code()
        .to_string()
}

#[test]
fn a_stripped_shell_body_runs_like_the_called_shell_in_every_mode() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs = rows("ref-");
    programs.extend(rows("dec-"));
    assert!(programs.len() >= 20, "found {} rows", programs.len());
    let default = |called: &str| run(options(true, false), called);
    let unsafe_lane = |called: &str| run(options(true, true), called);
    let plain = |called: &str| run(options(false, false), called);
    let modes: [Mode<'_>; 3] = [
        ("default", &default),
        ("unsafe", &unsafe_lane),
        ("no-deobfuscate", &plain),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_shell_body_without_own_level_bindings_stays_at_the_top_level() {
    let programs = rows("dec-");
    assert!(programs.len() >= 4, "found {} rows", programs.len());
    for (label, called) in &programs {
        for deobfuscate in [true, false] {
            let out = Pipeline::new(options(deobfuscate, false)).run(shell_of(called));
            assert!(
                out.notes
                    .iter()
                    .all(|note| note.message != ANONYMOUS_SHELL_SCOPED),
                "{label}: {:?}",
                out.notes
            );
        }
    }
}

#[test]
fn a_shell_body_that_declares_names_is_kept_inside_a_called_function() {
    for (label, called) in rows("ref-") {
        let out = Pipeline::new(options(false, false)).run(shell_of(&called));
        assert!(
            out.code().starts_with("(function () {"),
            "{label}: {}",
            out.code()
        );
        assert!(out.code().ends_with("})();"), "{label}: {}", out.code());
        assert!(
            out.notes
                .iter()
                .any(|note| note.pass == "strip-shell" && note.message == ANONYMOUS_SHELL_SCOPED),
            "{label}: {:?}",
            out.notes
        );
    }
}

#[test]
fn a_decoded_shell_whose_machinery_is_removed_needs_no_function_around_it() {
    let [(_, called)] = rows("ref-decoder").try_into().unwrap();
    let out = Pipeline::new(options(true, false)).run(shell_of(&called));
    assert!(!out.code().contains("function"), "{}", out.code());
    assert!(out.code().contains("\"Hello World!!\""), "{}", out.code());
}
