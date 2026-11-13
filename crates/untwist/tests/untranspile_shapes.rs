use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_unminify::untranspile::shapes;

const SHAPE_FIXTURES: &[(&str, &str, &str)] = &[
    (
        "untranspile-helpers",
        "array/concat/safe",
        "fixtures/untranspile/babel/spread.7.29.js",
    ),
    (
        "untranspile-helpers",
        "array/spread-array-read/unsafe",
        "fixtures/untranspile/ts/class-fields.es5-iter.js",
    ),
    (
        "untranspile-helpers",
        "array/spread-array/safe",
        "fixtures/untranspile/ts/spread.es5-iter.js",
    ),
    (
        "untranspile-helpers",
        "array/spread-array/unsafe",
        "fixtures/untranspile/samples/ts-es5.js",
    ),
    (
        "untranspile-helpers",
        "array/to-consumable-array/safe",
        "fixtures/untranspile/babel/spread.7.29.js",
    ),
    (
        "untranspile-helpers",
        "array/to-consumable-array/unsafe",
        "fixtures/untranspile/babel/generators.7.29.js",
    ),
    (
        "untranspile-helpers",
        "object/assign/safe",
        "fixtures/untranspile/hand/assign-proto-source.js",
    ),
    (
        "untranspile-helpers",
        "object/assign/unsafe",
        "fixtures/untranspile/hand/assign-proto-source.js",
    ),
    (
        "untranspile-helpers",
        "object/define-chain/safe",
        "fixtures/untranspile/babel/define-chain.7.29.js",
    ),
    (
        "untranspile-helpers",
        "object/define-chain-run/safe",
        "fixtures/untranspile/babel/define-chain-hand.js",
    ),
    (
        "untranspile-helpers",
        "object/object-spread-each/unsafe",
        "fixtures/untranspile/swc/spread.es2015.js",
    ),
    (
        "untranspile-helpers",
        "object/object-spread2/safe",
        "fixtures/untranspile/babel/spread.7.29.js",
    ),
    (
        "untranspile-helpers",
        "object/object-spread2/unsafe",
        "fixtures/untranspile/babel/spread.7.29.js",
    ),
    (
        "untranspile-helpers",
        "object/spread-props/safe",
        "fixtures/untranspile/esbuild/spread.0.25.12-es2015.js",
    ),
    (
        "untranspile-helpers",
        "object/spread-values/safe",
        "fixtures/untranspile/esbuild/spread.0.25.12-es2015.js",
    ),
    (
        "untranspile-helpers",
        "object/spread-values/unsafe",
        "fixtures/untranspile/esbuild/spread.0.25.12-es2015.js",
    ),
    (
        "untranspile-helpers",
        "sweep/declarator/unsafe",
        "fixtures/untranspile/esbuild/spread.0.25.12-es2015.js",
    ),
    (
        "untranspile-helpers",
        "sweep/function/safe",
        "fixtures/untranspile/terser/spread.safari9.js",
    ),
    (
        "untranspile-helpers",
        "sweep/function/unsafe",
        "fixtures/untranspile/babel/generators.7.29.js",
    ),
    (
        "untranspile-helpers",
        "destructure/read/safe",
        "fixtures/untranspile/ts/rest.es5-iter.js",
    ),
    (
        "untranspile-helpers",
        "destructure/read/unsafe",
        "fixtures/untranspile/ts/destructure.es5-iter.js",
    ),
    (
        "untranspile-helpers",
        "destructure/sliced-to-array/safe",
        "fixtures/untranspile/hand/destructure-temp.js",
    ),
    (
        "untranspile-helpers",
        "destructure/sliced-to-array/unsafe",
        "fixtures/untranspile/babel/destructure.7.29.js",
    ),
    (
        "untranspile-helpers",
        "class/iife/base/unsafe",
        "fixtures/untranspile/babel/classes.7.29.js",
    ),
    (
        "untranspile-helpers",
        "class/iife/derived/unsafe",
        "fixtures/untranspile/babel/classes.7.29.js",
    ),
    (
        "untranspile-helpers",
        "class/direct/unsafe",
        "fixtures/untranspile/babel/classes.7.29.js",
    ),
    (
        "untranspile-helpers",
        "class/field/unsafe",
        "fixtures/behaviour/class-shapes.js",
    ),
    (
        "untranspile-helpers",
        "class/native/base/unsafe",
        "fixtures/untranspile/esbuild/class-fields.0.28.1-es2015.js",
    ),
    (
        "untranspile-helpers",
        "class/native/derived/unsafe",
        "fixtures/untranspile/esbuild/class-fields.0.28.1-es2015.js",
    ),
    (
        "untranspile-helpers",
        "generator/tslib/unsafe",
        "fixtures/untranspile/ts/generators.es5.js",
    ),
    (
        "untranspile-helpers",
        "async/awaiter/unsafe",
        "fixtures/untranspile/ts/async.es2015.js",
    ),
    (
        "untranspile-helpers",
        "async/esbuild/unsafe",
        "fixtures/untranspile/esbuild/async.0.28.1-es2015.js",
    ),
    (
        "merge-member-declarations",
        "member-run",
        "fixtures/untranspile/hand/merge-member-declarations.js",
    ),
    (
        "var-to-let",
        "statement",
        "fixtures/behaviour/var-let-shapes.js",
    ),
    (
        "var-to-let",
        "for-head",
        "fixtures/behaviour/var-let-shapes.js",
    ),
    (
        "var-to-let",
        "for-in-of-head",
        "fixtures/behaviour/var-let-shapes.js",
    ),
    (
        "var-to-let",
        "placed-at-first-write",
        "fixtures/untranspile/terser/spread.safari9.js",
    ),
    (
        "var-to-let",
        "hoisted-to-top",
        "fixtures/untranspile/terser/spread.safari9.js",
    ),
    (
        "function-to-arrow",
        "called-binding",
        "fixtures/behaviour/arrow-shapes.js",
    ),
    (
        "function-to-arrow",
        "callback-argument",
        "fixtures/behaviour/arrow-shapes.js",
    ),
];

const NODE_CHECKED_DIRS: [&str; 2] = ["fixtures/untranspile/", "fixtures/behaviour/"];

const FIXTURE_DIRS: [&str; 2] = ["fixtures/untranspile", "fixtures/behaviour"];

const CORPUS_DIRS: [&str; 6] = [
    "fixtures/corpus-npm",
    "fixtures/unminify",
    "fixtures/jsx",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/real-world",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn js_below(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            js_below(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
}

fn files_in(dirs: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in dirs {
        js_below(&root().join(dir), &mut files);
    }
    files.sort();
    files
}

fn modes() -> [Options; 4] {
    let safe = Options::default();
    let conservative = Options {
        extended: false,
        ..Options::default()
    };
    let unsafe_mode = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let unsafe_conservative = Options {
        unsafe_transforms: true,
        extended: false,
        ..Options::default()
    };
    [safe, conservative, unsafe_mode, unsafe_conservative]
}

fn is_covered(pass: &str, shape: &str) -> bool {
    SHAPE_FIXTURES
        .iter()
        .any(|(row_pass, row_shape, _)| *row_pass == pass && *row_shape == shape)
}

fn uncovered_shapes_of(files: &[PathBuf]) -> BTreeMap<shapes::Shape, String> {
    shapes::take();
    let mut uncovered = BTreeMap::new();
    for file in files {
        let source = std::fs::read_to_string(file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        for options in modes() {
            Pipeline::new(options).run(&source);
        }
        for (pass, shape) in shapes::take().into_keys() {
            if !is_covered(pass, shape) {
                uncovered
                    .entry((pass, shape))
                    .or_insert_with(|| file.display().to_string());
            }
        }
    }
    uncovered
}

fn assert_fired_shapes_are_covered(files: &[PathBuf]) {
    let uncovered = uncovered_shapes_of(files);
    assert!(
        uncovered.is_empty(),
        "these (pass, shape) pairs fired with no node-checked fixture row, each with a file \
         that fires it: {uncovered:#?}"
    );
}

#[test]
fn every_shape_row_names_a_node_checked_fixture() {
    for (pass, shape, fixture) in SHAPE_FIXTURES {
        assert!(
            NODE_CHECKED_DIRS.iter().any(|dir| fixture.starts_with(dir)),
            "{pass}/{shape} names {fixture}, which no node-checked lane runs"
        );
        assert!(
            root().join(fixture).is_file(),
            "{pass}/{shape} names {fixture}, which does not exist"
        );
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "about 1,700 pipeline runs; run with cargo test --release -p untwist --test untranspile_shapes"
)]
fn the_node_checked_fixtures_fire_only_covered_shapes() {
    assert_fired_shapes_are_covered(&files_in(&FIXTURE_DIRS));
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "game.js in four modes; run with cargo test --release -p untwist --test untranspile_shapes"
)]
fn the_corpus_the_diff_sets_and_game_js_fire_only_covered_shapes() {
    assert_fired_shapes_are_covered(&files_in(&CORPUS_DIRS));
}
