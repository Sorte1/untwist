use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use untwist::{Options, Pipeline};
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_core::reflow::{reflow_recorded, Reflower};

const GUARD: &str = "var ctrl = function () {
  var f = !![];
  return function (c, fn) {
    var r = f ? function () {
        if (fn) {
          var res = fn.apply(c, arguments);
          fn = null;
          return res;
        }
      } : function () {};
    f = ![];
    return r;
  };
}();
";

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn reprinted(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    pretty_codegen(&parsed.program)
}

fn large_module() -> String {
    (0..4_000)
        .map(|index| format!("var f{index} = function() {{ return [{index}, 2, 3, 4]; }};\n"))
        .collect()
}

fn assert_one_arena_matches_fresh_arenas(texts: &[String]) {
    let mut reflower = Reflower::default();
    for (index, text) in texts.iter().enumerate() {
        let reused = reflower.reflow_recorded(text.clone());
        let fresh = reflow_recorded(text.clone());
        assert_eq!(reused, fresh, "text {index}");
    }
}

#[test]
fn a_module_reflowed_after_the_arena_reset_matches_its_fresh_arena_form() {
    let texts = [
        large_module(),
        reprinted("var a = function() { return [1, 2, 3]; };"),
        reprinted(GUARD),
        "var broken = function( {".to_string(),
        reprinted(&GUARD.replace("var ctrl", "var other")),
        large_module(),
        reprinted("var b = async function() { return [4,\n5, 6]; };"),
    ];
    assert_one_arena_matches_fresh_arenas(&texts);
}

#[test]
fn a_reused_arena_records_the_same_guards_as_fresh_arenas() {
    let mut reflower = Reflower::default();
    let (_, first) = reflower.reflow_recorded(large_module());
    let (out, record) = reflower.reflow_recorded(reprinted(GUARD));
    assert_eq!(first, reflow_recorded(large_module()).1);
    assert_eq!((out, record), reflow_recorded(reprinted(GUARD)));
}

#[test]
fn every_unpack_fixture_module_reflows_the_same_through_one_arena() {
    let options = Options {
        deobfuscate: false,
        unminify: false,
        jsx: false,
        ..Options::default()
    };
    let mut texts = Vec::new();
    for path in unpack_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read an unpack fixture");
        let Some(bundle) = Pipeline::new(options.clone()).run(&source).bundle else {
            continue;
        };
        texts.extend(bundle.modules.iter().map(|module| reprinted(&module.code)));
    }
    assert!(texts.len() >= 20, "{}", texts.len());
    assert_one_arena_matches_fresh_arenas(&texts);
}

fn unpack_fixtures() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(workspace("fixtures/reference/unpack"))
        .expect("the unpack fixtures")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    paths.sort();
    paths
}

fn module_codes(source: &str, jobs: Option<usize>, base: &Options) -> Option<Vec<String>> {
    let options = Options {
        jobs: jobs.and_then(NonZeroUsize::new),
        ..base.clone()
    };
    let bundle = Pipeline::new(options).run(source).bundle?;
    Some(
        bundle
            .modules
            .into_iter()
            .map(|module| module.code)
            .collect(),
    )
}

fn assert_widths_agree(source: &str, base: &Options, label: &str) {
    let default = module_codes(source, None, base);
    assert_eq!(module_codes(source, Some(1), base), default, "{label} -j 1");
    assert_eq!(module_codes(source, Some(4), base), default, "{label} -j 4");
}

#[test]
fn every_unpack_fixture_agrees_at_one_four_and_the_default_width() {
    let base = Options::default();
    for path in unpack_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read an unpack fixture");
        assert_widths_agree(&source, &base, &path.display().to_string());
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 5 MB bundle; run with cargo test --release -p untwist --test reflow_reuse"
)]
fn the_five_megabyte_bundle_agrees_at_one_four_and_the_default_width() {
    let path = workspace("fixtures/generated/webpack-5mb.js");
    let Ok(source) = std::fs::read_to_string(&path) else {
        assert!(
            std::env::var("UNTWIST_REQUIRE_BIG_BUNDLE").map_or(true, |value| value.is_empty()),
            "UNTWIST_REQUIRE_BIG_BUNDLE is set but fixtures/generated/webpack-5mb.js is missing"
        );
        return;
    };
    let base = Options {
        deobfuscate: false,
        unminify: false,
        jsx: false,
        ..Options::default()
    };
    assert_widths_agree(&source, &base, "webpack-5mb");
}
