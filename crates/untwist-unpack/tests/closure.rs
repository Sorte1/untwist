use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_testing::js_files_in;
use untwist_unpack::{holds_a_bundle, unpack, unpack_taking, BundleKind, UnpackResult};

type Files = Vec<(String, String)>;

const FIXTURES: &str = "fixtures/unpack-bundlers/closure";

fn wrap(segments: &str) -> String {
    format!(
        "this._s = this._s || {{}};\n(function (_) {{\n  var window = this;\n{segments}\n}}).call(this, this._s);"
    )
}

fn setup() -> &'static str {
    "try { _._ModuleManager_initialize = function () {}; } catch (e) { _._DumpException(e); }"
}

fn segment(id: &str, body: &str) -> String {
    format!("try {{ /*_M:{id}*/ {body} }} catch (e) {{ _._DumpException(e); }}")
}

fn bundle(segments: &[String]) -> String {
    wrap(&format!("{}\n{}", setup(), segments.join("\n")))
}

fn files(result: Option<UnpackResult<'_>>) -> Option<Files> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Closure);
    Some(
        result
            .bundle
            .modules
            .values()
            .map(|module| {
                (
                    module.path.as_str().to_string(),
                    pretty_codegen(module.program()),
                )
            })
            .collect(),
    )
}

fn split(source: &str) -> Option<Files> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    files(unpack(&alloc, &parsed.program, &[]))
}

fn split_taken(source: &str) -> (Option<Files>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = files(unpack_taking(&alloc, &mut parsed.program, &[]));
    (found, pretty_codegen(&parsed.program))
}

fn code<'f>(files: &'f Files, path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, code)| code.as_str())
        .unwrap_or_else(|| {
            panic!(
                "no {path} in {:?}",
                files.iter().map(|(p, _)| p).collect::<Vec<_>>()
            )
        })
}

fn paths(files: &Files) -> Vec<&str> {
    files.iter().map(|(path, _)| path.as_str()).collect()
}

fn refused(source: &str) {
    assert_eq!(split(source), None, "split:\n{source}");
    let alloc = Allocator::default();
    let before = pretty_codegen(&parse_program(&alloc, source).program);
    let (taken, after) = split_taken(source);
    assert_eq!(taken, None);
    assert_eq!(after, before, "the refused input is left intact");
}

#[test]
fn each_marked_segment_becomes_a_module_and_the_entry_requires_them_in_order() {
    let files = split(&bundle(&[
        segment("a", "_.a = 1;"),
        segment("b", "console.log(_.a);"),
    ]))
    .expect("a closure bundle");
    assert_eq!(paths(&files), ["a.js", "b.js", "index.js"]);
    let a = code(&files, "a.js");
    assert!(a.contains("globalThis._s = globalThis._s || {};"), "{a}");
    assert!(
        a.contains("var window = this;") && a.contains("_.a = 1;"),
        "{a}"
    );
    assert!(a.contains(".call(globalThis, globalThis._s)"), "{a}");
    assert!(!a.contains("console.log"), "{a}");
    let index = code(&files, "index.js");
    assert!(index.contains("_ModuleManager_initialize"), "{index}");
    assert!(
        index.find("require(\"./a.js\")") < index.find("require(\"./b.js\")")
            && index.contains("require(\"./a.js\")"),
        "{index}"
    );
}

#[test]
fn taking_equals_cloning_and_leaves_the_wrapper_empty() {
    let source = bundle(&[segment("a", "_.a = 1;"), segment("b", "_.b = _.a;")]);
    let cloned = split(&source).expect("cloned");
    let (taken, rest) = split_taken(&source);
    assert_eq!(taken, Some(cloned));
    assert!(!rest.contains("_.a = 1"), "{rest}");
}

#[test]
fn a_marker_comment_stays_with_its_module() {
    let files = split(&bundle(&[segment("a", "_.a = 1;")])).expect("closure");
    assert!(code(&files, "a.js").contains("_M:a"));
}

#[test]
fn an_unmarked_segment_after_a_marked_one_refuses() {
    refused(&bundle(&[
        segment("a", "_.a = 1;"),
        "try { _.b = 2; } catch (e) { _._DumpException(e); }".to_string(),
    ]));
}

#[test]
fn duplicate_ids_refuse() {
    refused(&bundle(&[
        segment("a", "_.a = 1;"),
        segment("a", "_.b = 1;"),
    ]));
}

#[test]
fn an_id_that_could_leave_the_directory_refuses() {
    for id in ["..", "a/b", "closure", "-x"] {
        refused(&bundle(&[segment(id, "_.a = 1;")]));
    }
}

#[test]
fn a_segment_reading_another_segments_function_refuses() {
    refused(&bundle(&[
        segment(
            "a",
            "var shared = function () { return 1; }; var s = { f: function g() {} };",
        ),
        segment("b", "var g = 1; console.log(shared(), g);"),
        segment(
            "c",
            "if (_.x) { var h = 1; } function shared2() {} _.f = shared2;",
        ),
        segment("d", "console.log(shared2);"),
    ]));
}

#[test]
fn a_segment_reading_a_bundle_scope_binding_refuses() {
    let source = format!(
        "var outer = 1;\n{}",
        bundle(&[segment("a", "console.log(outer);")])
    );
    refused(&source);
}

#[test]
fn a_top_level_return_in_a_segment_refuses() {
    refused(&bundle(&[
        segment("a", "if (_.a) return; _.a = 1;"),
        segment("b", "_.b = 1;"),
    ]));
}

#[test]
fn a_segment_reading_a_host_name_free_refuses() {
    for read in [
        "__filename",
        "__dirname",
        "module.filename",
        "require(\"path\")",
        "exports",
    ] {
        refused(&bundle(&[
            segment("a", "_.a = 1;"),
            segment("b", &format!("console.log({read});")),
        ]));
    }
    let local = bundle(&[segment("a", "var module = 1; console.log(module);")]);
    assert!(split(&local).is_some());
}

#[test]
fn a_return_inside_a_nested_function_is_fine() {
    let files = split(&bundle(&[segment("a", "_.f = function () { return 1; };")]));
    assert!(files.is_some());
}

#[test]
fn writing_the_namespace_parameter_or_a_prelude_var_refuses() {
    refused(&bundle(&[segment("a", "_ = {};")]));
    refused(&bundle(&[segment("a", "window = 1;")]));
}

#[test]
fn a_shell_that_does_not_share_one_namespace_refuses() {
    let source = bundle(&[segment("a", "_.a = 1;")]).replace("this._s)", "this._t)");
    refused(&source);
    let source = bundle(&[segment("a", "_.a = 1;")]).replace(".call(this,", ".call(null,");
    refused(&source);
}

#[test]
fn a_catch_that_does_not_dump_the_exception_refuses() {
    let other = "try { /*_M:a*/ _.a = 1; } catch (e) { console.log(e); }".to_string();
    refused(&bundle(&[other]));
    let finalizer =
        "try { /*_M:a*/ _.a = 1; } catch (e) { _._DumpException(e); } finally { _.done = 1; }"
            .to_string();
    refused(&bundle(&[finalizer]));
}

#[test]
fn an_unguarded_top_level_statement_refuses() {
    let source =
        bundle(&[segment("a", "_.a = 1;")]).replace("\n}).call", "\nconsole.log(1);\n}).call");
    refused(&source);
}

#[test]
fn a_prelude_var_that_is_not_this_refuses() {
    let source =
        bundle(&[segment("a", "_.a = 1;")]).replace("var window = this;", "var window = 1;");
    refused(&source);
}

#[test]
fn two_initializer_calls_or_none_refuse() {
    let twice = wrap(
        "try { _._ModuleManager_initialize = function () {}; _._ModuleManager_initialize(\"a:0\", []); } catch (e) { _._DumpException(e); }\ntry { _._ModuleManager_initialize(\"b:0\", []); } catch (e) { _._DumpException(e); }\ntry { /*_M:a*/ _.a = 1; } catch (e) { _._DumpException(e); }",
    );
    refused(&twice);
    let none = wrap(&segment("a", "_.a = 1;"));
    refused(&none);
}

#[test]
fn a_second_marker_in_one_segment_refuses() {
    refused(&bundle(&[segment("a", "/*_M:b*/ _.a = 1;")]));
}

#[test]
fn a_wrapper_with_a_use_strict_directive_keeps_it_in_every_module() {
    let source = bundle(&[segment("a", "_.a = 1;")]).replace(
        "var window = this;",
        "\"use strict\";\n  var window = this;",
    );
    let files = split(&source).expect("closure");
    assert!(code(&files, "a.js").contains("\"use strict\""));
}

#[test]
fn the_fixtures_split_taken_and_cloned_alike() {
    let mut accepted = 0;
    for (label, source) in js_files_in(FIXTURES) {
        let cloned = split(&source);
        assert_eq!(split_taken(&source).0, cloned, "{label}");
        accepted += usize::from(cloned.is_some());
        if cloned.is_none() {
            refused(&source);
        }
    }
    assert!(accepted >= 2, "only {accepted} closure fixtures split");
}

#[test]
fn the_probe_accepts_a_closure_bundle_and_not_a_plain_iife() {
    let alloc = Allocator::default();
    let text = bundle(&[segment("a", "_.a = 1;")]);
    let yes = parse_program(&alloc, &text);
    assert!(holds_a_bundle(&yes.program));
    let no = parse_program(
        &alloc,
        "(function (_) { try { _.a = 1; } catch (e) { throw e; } }).call(this, {});",
    );
    assert!(!holds_a_bundle(&no.program));
}

#[test]
fn a_prelude_var_after_the_first_segment_refuses() {
    let source =
        bundle(&[segment("a", "_.a = 1;")]).replace("\n}).call", "\nvar late = this;\n}).call");
    refused(&source);
}

#[test]
fn a_bundle_with_setup_but_no_marked_segment_refuses() {
    refused(&wrap(setup()));
}

#[test]
fn every_module_repeats_the_prelude_and_the_entry_holds_the_setup_once() {
    let files = split(&bundle(&[
        segment("a", "_.a = 1;"),
        segment("b", "_.b = 2;"),
    ]))
    .expect("closure");
    for path in ["a.js", "b.js"] {
        assert_eq!(code(&files, path).matches("var window = this;").count(), 1);
        assert!(!code(&files, path).contains("_ModuleManager_initialize"));
    }
    assert_eq!(
        code(&files, "index.js")
            .matches("_ModuleManager_initialize")
            .count(),
        1
    );
}

#[test]
fn a_namespace_initialised_from_another_property_refuses() {
    let source = bundle(&[segment("a", "_.a = 1;")]).replace("this._s || {}", "this._t || {}");
    refused(&source);
}

#[test]
fn a_catch_that_dumps_on_another_object_or_another_value_refuses() {
    let other_object =
        "try { /*_M:a*/ _.a = 1; } catch (e) { window._DumpException(e); }".to_string();
    refused(&bundle(&[other_object]));
    let other_value = "try { /*_M:a*/ _.a = 1; } catch (e) { _._DumpException(_); }".to_string();
    refused(&bundle(&[other_value]));
}

#[test]
fn a_named_or_async_wrapper_refuses() {
    let named = bundle(&[segment("a", "_.a = 1;")]).replace("(function (_)", "(function wrap(_)");
    refused(&named);
    let asynchronous =
        bundle(&[segment("a", "_.a = 1;")]).replace("(function (_)", "(async function (_)");
    refused(&asynchronous);
}

#[test]
fn an_empty_marker_refuses() {
    refused(&bundle(&[segment("", "_.a = 1;")]));
}

#[test]
fn the_entry_requires_each_module_once_and_not_itself() {
    let files = split(&bundle(&[
        segment("a", "_.a = 1;"),
        segment("b", "_.b = 2;"),
    ]))
    .expect("closure");
    let index = code(&files, "index.js");
    assert_eq!(index.matches("require(").count(), 2, "{index}");
    assert!(!index.contains("index.js"), "{index}");
}

fn split_both(source: &str) -> Files {
    let cloned = split(source).expect("a closure bundle");
    let (taken, rest) = split_taken(source);
    assert_eq!(taken.as_ref(), Some(&cloned), "taken differs from cloned");
    assert!(!rest.contains("console.log"), "{rest}");
    cloned
}

#[test]
fn a_block_function_another_segment_reads_refuses() {
    refused(&bundle(&[
        segment("a", "function shared() { return 1; }"),
        segment("b", "console.log(shared());"),
    ]));
    refused(&bundle(&[
        segment("a", "if (_.x) { function inner() {} }"),
        segment("b", "console.log(typeof inner);"),
    ]));
}

#[test]
fn a_block_function_used_only_in_its_segment_splits() {
    let files = split_both(&bundle(&[
        segment("a", "function own() { return 1; } _.a = own();"),
        segment("b", "console.log(_.a);"),
    ]));
    assert_eq!(paths(&files), ["a.js", "b.js", "index.js"]);
}

#[test]
fn a_var_shared_across_segments_moves_to_the_holder() {
    let files = split_both(&bundle(&[
        segment("a", "var shared = function (x) { return x + 1; }, own = 2; var later; console.log(own);"),
        segment("b", "shared = shared; later = shared(own2()); function own2() { return 3; } console.log(later, typeof shared, `${shared(1)}`);"),
        segment("c", "console.log(later, shared(2), shared.call(null, 3)); later++; later += 1; for (later in { k: 1 }) {} new shared(0);"),
    ]));
    assert_eq!(
        paths(&files),
        ["a.js", "b.js", "c.js", "index.js", "_runtime/closure.js"]
    );
    let a = code(&files, "a.js");
    assert!(
        a.starts_with("const __closure_vars = require(\"./_runtime/closure.js\");"),
        "{a}"
    );
    assert!(a.contains("var own;"), "{a}");
    assert!(a.contains("__closure_vars.shared = function(x)"), "{a}");
    assert!(a.contains("own = 2"), "{a}");
    assert!(!a.contains("var later"), "{a}");
    let b = code(&files, "b.js");
    assert!(
        b.contains("__closure_vars.later = (0, __closure_vars.shared)(own2());"),
        "{b}"
    );
    assert!(b.contains("typeof __closure_vars.shared"), "{b}");
    let c = code(&files, "c.js");
    assert!(c.contains("__closure_vars.later++"), "{c}");
    assert!(c.contains("for (__closure_vars.later in"), "{c}");
    assert!(c.contains("new __closure_vars.shared(0)"), "{c}");
    assert!(c.contains("__closure_vars.shared.call(null, 3)"), "{c}");
    let index = code(&files, "index.js");
    assert!(
        index.contains("require(\"./_runtime/closure.js\")"),
        "{index}"
    );
    assert_eq!(index.matches("_runtime/closure.js").count(), 1, "{index}");
    let holder = code(&files, "_runtime/closure.js");
    assert!(
        holder.contains("module.exports = Object.create(null);"),
        "{holder}"
    );
}

#[test]
fn the_holder_name_avoids_the_source_text() {
    let files = split_both(&bundle(&[
        segment(
            "a",
            "var shared = 1; var __closure_vars = 2; console.log(__closure_vars);",
        ),
        segment("b", "console.log(shared);"),
    ]));
    assert!(code(&files, "b.js").contains("__closure_vars2.shared"));
}

#[test]
fn a_shared_var_in_an_unsupported_position_refuses() {
    for (a, b) in [
        (
            "for (var shared = 0; shared < 1; shared++) {}",
            "console.log(shared);",
        ),
        ("for (var shared in {}) {}", "console.log(shared);"),
        ("var { shared } = { shared: 1 };", "console.log(shared);"),
        ("var shared = 1, [x] = [2];", "console.log(shared, x);"),
        ("var shared = 1;", "console.log({ shared });"),
        ("var shared = 1;", "({ shared } = { shared: 2 });"),
        ("var shared = 1;", "[shared] = [2];"),
        ("var shared = 1;", "console.log(delete shared);"),
        ("var shared = 1;", "with (_) { console.log(shared); }"),
        ("var shared = 1; with (_) {}", "console.log(shared);"),
    ] {
        refused(&bundle(&[segment("a", a), segment("b", b)]));
    }
}

#[test]
fn a_shared_var_read_by_setup_moves_too() {
    let source = wrap(&format!(
        "try {{ _._ModuleManager_initialize = function () {{}}; var early = function () {{ return later; }}; }} catch (e) {{ _._DumpException(e); }}\n{}\n{}",
        segment("a", "var later = 5;"),
        segment("b", "console.log(early());"),
    ));
    let files = split_both(&source);
    assert!(code(&files, "index.js").contains("__closure_vars.early = function()"));
    assert!(code(&files, "a.js").contains("__closure_vars.later = 5;"));
}

fn loader_segment(id: &str, body: &str) -> String {
    format!("try {{ _.l(\"{id}\"); {body} _.m(); }} catch (e) {{ _._DumpException(e); }}")
}

fn graph_setup(graph: &str) -> String {
    format!("try {{ _._ModuleManager_initialize = function () {{}}; _._ModuleManager_initialize(\"{graph}\", []); }} catch (e) {{ _._DumpException(e); }}")
}

#[test]
fn loader_calls_name_unmarked_segments() {
    let source = wrap(&format!(
        "{}\n{}\n{}",
        graph_setup("base/sy1:0/main:0,1"),
        loader_segment("sy1", "_.a = 1;"),
        loader_segment("main", "console.log(_.a);"),
    ));
    let files = split_both(&source);
    assert_eq!(paths(&files), ["sy1.js", "main.js", "index.js"]);
    let plain = wrap(&format!("{}\n{}", setup(), loader_segment("x", "_.a = 1;")));
    assert_eq!(paths(&split_both(&plain)), ["x.js", "index.js"]);
    let marked = wrap(&format!(
        "{}\n{}",
        setup(),
        segment("x", "_.l(\"x\"); _.a = 1; _.m();")
    ));
    assert_eq!(paths(&split_both(&marked)), ["x.js", "index.js"]);
}

#[test]
fn inconsistent_loader_ids_refuse() {
    for segments in [
        vec![
            graph_setup("base/sy1:0"),
            loader_segment("main", "_.a = 1;"),
        ],
        vec![
            setup().to_string(),
            loader_segment("a", ""),
            "try { _.k(\"b\"); _.m(); } catch (e) { _._DumpException(e); }".to_string(),
        ],
        vec![
            setup().to_string(),
            loader_segment("a", ""),
            "try { _.l(\"b\"); _.n(); } catch (e) { _._DumpException(e); }".to_string(),
        ],
        vec![setup().to_string(), segment("a", "_.l(\"b\"); _.m();")],
        vec![setup().to_string(), loader_segment("../x", "")],
        vec![
            setup().to_string(),
            loader_segment("a", ""),
            loader_segment("a", ""),
        ],
        vec![
            setup().to_string(),
            "try { _.l(\"a\"); _.l(); } catch (e) { _._DumpException(e); }".to_string(),
        ],
        vec![
            setup().to_string(),
            "try { _.l(\"a\", 1); _.m(); } catch (e) { _._DumpException(e); }".to_string(),
        ],
    ] {
        refused(&wrap(&segments.join("\n")));
    }
}
