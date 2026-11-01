mod common;

use std::path::{Path, PathBuf};

use common::prepared;
use oxc_allocator::Allocator;
use untwist_unpack::detect::{holds_a_bundle, unpack_program};

const BUNDLERS: &str = "../../fixtures/unpack-bundlers";

const MATRIX: &[(&str, Option<&str>)] = &[
    ("amd/min-amd-named.js", Some("amd")),
    ("amd/min-amd.js", Some("amd")),
    ("amd/neg-amd-cjs-wrapper.js", None),
    ("amd/neg-amd-cycle.js", None),
    ("amd/neg-amd-early-return.js", None),
    ("amd/neg-amd-host-read.js", None),
    ("amd/neg-amd-mixed.js", None),
    ("amd/neg-amd-require-dep.js", None),
    ("amd/neg-amd-spread-deps.js", None),
    ("amd/neg-amd-this.js", None),
    ("amd/neg-amd-top-level-code.js", None),
    ("amd/rollup-amd.js", Some("amd")),
    ("bun/bun-cjs.js", Some("esbuild")),
    ("bun/bun-node.js", Some("esbuild")),
    ("bun/min-bun-cjs.js", Some("esbuild")),
    ("closure/min-closure-loader.js", Some("closure")),
    ("closure/min-closure-shared-var.js", Some("closure")),
    ("closure/min-closure-three.js", Some("closure")),
    ("closure/min-closure.js", Some("closure")),
    ("closure/neg-closure-annex-b.js", None),
    ("closure/neg-closure-duplicate-ids.js", None),
    ("closure/neg-closure-two-initializers.js", None),
    ("closure/neg-closure-unguarded.js", None),
    ("closure/neg-closure-unmarked.js", None),
    ("closure/neg-closure-late-unmarked.js", None),
    ("closure/neg-closure-loader-graph.js", None),
    ("closure/neg-closure-param-write.js", None),
    ("closure/neg-closure-return.js", None),
    ("closure/neg-closure-shared-var.js", None),
    ("closure/neg-closure-shell.js", None),
    ("esbuild/esbuild-cjs-min.js", Some("esbuild")),
    ("esbuild/esbuild-cjs.js", Some("esbuild")),
    ("esbuild/esbuild-esm-min.js", Some("esbuild")),
    ("esbuild/esbuild-esm.js", Some("esbuild")),
    ("esbuild/esbuild-iife-min.js", Some("esbuild")),
    ("esbuild/esbuild-iife.js", Some("esbuild")),
    ("esbuild/min-esbuild-cjs.js", Some("esbuild")),
    ("esbuild/min-esbuild-esm.js", Some("esbuild")),
    ("esbuild/min-esbuild-in-amd.js", Some("esbuild")),
    ("esbuild/min-esbuild-in-system.js", Some("esbuild")),
    ("esbuild/min-esbuild-in-umd.js", Some("esbuild")),
    ("esbuild/min-esbuild-runtime.js", Some("esbuild")),
    ("esbuild/neg-esbuild-lazy-only.js", None),
    ("metro/metro.js", Some("metro")),
    ("metro/min-metro-interop.js", Some("metro")),
    ("metro/min-metro.js", Some("metro")),
    ("metro/neg-metro-bad-call.js", None),
    ("metro/neg-metro-five-params.js", None),
    ("parcel/hoist/index.js", None),
    ("parcel/hoist/lazy.58e54a87.js", None),
    ("parcel/min-parcel-entries.js", Some("parcel")),
    ("parcel/min-parcel-hoist-seq.js", Some("parcel")),
    ("parcel/min-parcel-hoist.js", Some("parcel")),
    ("parcel/min-parcel-nohoist.js", Some("parcel")),
    ("parcel/neg-parcel-bundle-binding.js", None),
    ("parcel/neg-parcel-chunk-load.js", None),
    ("parcel/neg-parcel-foreign-id.js", None),
    ("parcel/neg-parcel-late-register.js", None),
    ("parcel/neg-parcel-no-registry.js", None),
    ("parcel/nohoist/index.js", None),
    ("parcel/nohoist/lazy.99d34c29.js", None),
    ("rollup/min-rollup-forward.js", None),
    ("parcel/single-hoist-min.js", Some("parcel")),
    ("parcel/single-hoist.js", Some("parcel")),
    ("parcel/single-nohoist-min.js", Some("parcel")),
    ("parcel/single-nohoist.js", Some("parcel")),
    ("rollup/min-rollup-hoisted.js", None),
    ("rollup/neg-rollup-cross-write.js", None),
    ("rollup/neg-rollup-host-module.js", None),
    ("rollup/neg-rollup-one-module.js", None),
    ("rollup/neg-rollup-sloppy.js", None),
    ("rollup/rollup-iife.js", None),
    ("rollup/vite-iife.js", None),
    ("systemjs/min-system-named.js", Some("systemjs")),
    ("systemjs/min-system.js", Some("systemjs")),
    ("systemjs/neg-system-computed-deps.js", None),
    ("systemjs/neg-system-cycle.js", None),
    ("systemjs/neg-system-default-export-param.js", None),
    ("systemjs/neg-system-setter-shape.js", None),
    ("systemjs/neg-system-unsorted-entry.js", None),
    ("systemjs/rollup-system.js", Some("systemjs")),
    ("turbopack/browser-chunk.js", None),
    ("turbopack/min-entry/chunk.js", Some("turbopack")),
    ("turbopack/min-entry/index.js", None),
    ("turbopack/min-entry/runtime.js", None),
    ("turbopack/neg-entry/chunk.js", None),
    ("turbopack/neg-entry/index.js", None),
    ("turbopack/neg-entry/runtime.js", None),
    ("turbopack/min-server/index.js", None),
    ("turbopack/min-server/server/chunks/ssr/[turbopack]_runtime.js", None),
    ("turbopack/min-server/server/chunks/ssr/app.js", None),
    ("turbopack/min-server/server/chunks/ssr/lib.js", None),
    ("turbopack/neg-server/index.js", None),
    ("turbopack/neg-server/server/chunks/ssr/[turbopack]_runtime.js", None),
    ("turbopack/neg-server/server/chunks/ssr/app.js", None),
    ("turbopack/neg-server/server/chunks/ssr/lib.js", None),
    ("turbopack/server/index.js", None),
    (
        "turbopack/server/server/chunks/ssr/[externals]_path_1ulxq_vdi8_o7._.js",
        None,
    ),
    ("turbopack/server/server/chunks/ssr/[turbopack]_runtime.js", None),
    (
        "turbopack/server/server/chunks/ssr/node_modules__work_next_src_1am5ze_f8hydp._.js",
        None,
    ),
    (
        "turbopack/server/server/chunks/ssr/node_modules__work_next_src_1k301osa1_9sg._.js",
        None,
    ),
    (
        "turbopack/server/server/chunks/ssr/node_modules__work_next_src_data_json_[json]_cjs_0hzgw30-hqo9e._.js",
        None,
    ),
    ("umd/min-umd-deps.js", Some("umd")),
    ("umd/min-umd-factory-first.js", Some("umd")),
    ("umd/min-umd.js", Some("umd")),
    ("umd/neg-umd-extra-statement.js", None),
    ("umd/neg-umd-relative-require.js", None),
    ("umd/neg-umd-two-wrappers.js", None),
    ("umd/neg-umd-unknown-env.js", None),
    ("umd/rollup-umd.js", Some("umd")),
    ("webpack/min-webpack5-locals.js", Some("webpack")),
    ("webpack/min-webpack5-precision.js", Some("webpack")),
    ("webpack/webpack4-dev-paths.js", Some("webpack")),
    ("webpack/webpack4-externals.js", Some("webpack")),
    ("webpack/webpack5-dev.js", Some("webpack")),
    ("webpack/webpack5-prod-concat-min.js", Some("webpack")),
    ("webpack/webpack5-prod-concat.js", Some("webpack")),
    ("webpack/webpack5-prod-min.js", Some("webpack")),
    ("webpack/webpack5-prod.js", Some("webpack")),
];

const SWEEP: &[&str] = &[
    "fixtures/behaviour",
    "fixtures/eval",
    "fixtures/unminify",
    "fixtures/deobfuscate-gaps",
    "fixtures/jsx",
    "fixtures/corpus-npm",
];

const SWEEP_DETECTED: &[&str] = &[];

fn manifest_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLERS)
}

fn manifest_paths() -> Vec<String> {
    let text = std::fs::read_to_string(manifest_root().join("MANIFEST")).expect("read MANIFEST");
    let mut paths: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').next().unwrap_or_default().to_string())
        .collect();
    paths.sort();
    paths
}

fn detected(source: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let program = prepared(&alloc, source);
    unpack_program(&alloc, &program).map(|bundle| bundle.kind.as_str())
}

fn probed(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, source);
    holds_a_bundle(&parsed.program) || unpack_program(&alloc, &parsed.program).is_some()
}

#[test]
fn the_matrix_names_every_bundler_fixture() {
    let mut rows: Vec<String> = MATRIX.iter().map(|(p, _)| (*p).to_string()).collect();
    rows.sort();
    assert_eq!(rows, manifest_paths());
}

#[test]
fn every_bundler_fixture_is_detected_as_the_matrix_says() {
    let root = manifest_root();
    let mut wrong = Vec::new();
    for (path, want) in MATRIX {
        let source = std::fs::read_to_string(root.join(path)).expect("read fixture");
        let got = detected(&source);
        if got != *want {
            wrong.push(format!("{path}: want {want:?}, got {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn js_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read sweep dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            js_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "js") {
            out.push(path);
        }
    }
}

#[test]
fn no_input_of_the_sweep_holds_a_bundle() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in SWEEP {
        js_files(&repo.join(dir), &mut files);
    }
    files.extend(untwist_testing::real_world_path("game.js"));
    files.sort();
    let mut found = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).expect("read sweep input");
        if probed(&source) {
            let rel = file.strip_prefix(&repo).unwrap_or(file);
            found.push(rel.display().to_string());
        }
    }
    assert!(files.len() > 270, "sweep shrank to {}", files.len());
    assert_eq!(found, SWEEP_DETECTED);
}
