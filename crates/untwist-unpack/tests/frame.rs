mod common;

use std::path::Path;

use common::parse;
use oxc_allocator::Allocator;
use untwist_unpack::frame::{Frame, Shape};

fn shape_of(source: &str) -> (Option<Shape>, usize, usize) {
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let frame = Frame::of(&program);
    let shape = frame.wrapper.as_ref().map(|wrapper| wrapper.shape);
    (shape, frame.top.len(), frame.inner().len())
}

fn bundler_fixture(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-bundlers");
    std::fs::read_to_string(root.join(path)).expect("read fixture")
}

#[test]
fn the_frame_finds_one_wrapper_of_each_shape() {
    for (source, want) in [
        ("(function () { a(); b(); })();", Some(Shape::Iife)),
        ("!function () { a(); }();", Some(Shape::Iife)),
        ("var app = (() => { a(); })();", Some(Shape::Iife)),
        (
            "(function (_) { a(); }).call(this, this._s);",
            Some(Shape::Call),
        ),
        (
            "(function (root, factory) { factory(); })(this, function () { a(); });",
            Some(Shape::Umd),
        ),
        (
            "// @bun @bun-cjs\n(function(exports, require, module, __filename, __dirname) { a(); })",
            Some(Shape::Bun),
        ),
        ("(function(exports, require, module) { a(); })", None),
        ("(function () { a(); })(); (function () { b(); })();", None),
        ("define([], function () { return 1; });", None),
        ("a(); b();", None),
    ] {
        assert_eq!(shape_of(source).0, want, "{source}");
    }
}

#[test]
fn the_inner_list_is_the_wrapper_body_or_the_top_level() {
    assert_eq!(
        shape_of("x(); (function () { a(); b(); c(); })();"),
        (Some(Shape::Iife), 2, 3)
    );
    assert_eq!(shape_of("a(); b();"), (None, 2, 2));
}

#[test]
fn the_umd_factory_is_the_function_argument_at_either_end() {
    for source in [
        "(function (root, factory) { factory(); })(this, function (e) { a(); b(); });",
        "(function (factory, root) { factory(); })(function (e) { a(); b(); }, this);",
    ] {
        let alloc = Allocator::default();
        let program = parse(&alloc, source);
        let frame = Frame::of(&program);
        let factory = frame
            .wrapper
            .as_ref()
            .and_then(|wrapper| wrapper.umd_factory())
            .expect("a factory");
        assert_eq!(factory.body.statements.len(), 2, "{source}");
    }
}

#[test]
fn the_generated_wrappers_have_the_shapes_their_bundlers_emit() {
    for (path, want) in [
        ("esbuild/esbuild-iife.js", Some(Shape::Iife)),
        ("rollup/rollup-iife.js", Some(Shape::Iife)),
        ("rollup/vite-iife.js", Some(Shape::Iife)),
        ("umd/rollup-umd.js", Some(Shape::Umd)),
        ("bun/bun-cjs.js", Some(Shape::Bun)),
        ("closure/min-closure.js", Some(Shape::Call)),
        ("metro/metro.js", Some(Shape::Iife)),
        ("amd/rollup-amd.js", None),
        ("esbuild/esbuild-cjs.js", None),
    ] {
        let source = bundler_fixture(path);
        assert_eq!(shape_of(&source).0, want, "{path}");
    }
}

#[test]
fn a_sequence_around_the_wrapper_call_is_not_a_wrapper() {
    for source in [
        "f(), function () { a(); }();",
        "!(f(), function () { a(); }());",
        "void (f(), (function (root, factory) { factory(); })(this, function () { a(); }));",
    ] {
        assert_eq!(shape_of(source).0, None, "{source}");
    }
}

#[test]
fn the_bun_wrapper_needs_the_pragma_where_bun_reads_it() {
    let wrapper = "(function(exports, require, module, __filename, __dirname) { a(); })";
    for (head, want) in [
        ("// @bun @bun-cjs\n", Some(Shape::Bun)),
        ("// @bun @bytecode @bun-cjs\n", Some(Shape::Bun)),
        ("#!/usr/bin/env bun\n// @bun @bun-cjs\n", Some(Shape::Bun)),
        ("", None),
        ("// @bun\n", None),
        ("// @bun-cjs\n", None),
        ("// @bun\n// @bun-cjs\n", None),
        ("\n// @bun @bun-cjs\n", None),
        (" // @bun @bun-cjs\n", None),
        ("/* @bun @bun-cjs */\n", None),
        ("// x\n// @bun @bun-cjs\n", None),
    ] {
        let source = format!("{head}{wrapper}");
        assert_eq!(shape_of(&source).0, want, "{source:?}");
    }
    let asynchronous = "// @bun @bun-cjs\n(async function(exports, require, module, __filename, __dirname) { a(); })";
    assert_eq!(shape_of(asynchronous).0, None);
}
