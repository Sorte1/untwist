use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking, BundleKind, UnpackResult};

const HOLDER: &str = "(globalThis.TURBOPACK||(globalThis.TURBOPACK=[])).push";
const SCRIPT: &str = "\"object\"==typeof document?document.currentScript:void 0";
const LEAF: &str = "2,(e,t,r)=>{r.v=1}";

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/unpack-bundlers/turbopack")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

fn push(items: &str) -> String {
    format!("{HOLDER}([{SCRIPT},{items}]);\n")
}

fn register(ids: &str) -> String {
    push(&format!("{{otherChunks:[],runtimeModuleIds:[{ids}]}}"))
}

fn chunk(entry: &str) -> String {
    format!("{}{}", push(&format!("1,{entry},{LEAF}")), register("1"))
}

fn files(result: Option<UnpackResult<'_>>) -> Option<Vec<(String, String)>> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Turbopack);
    let mut files: Vec<(String, String)> = result
        .bundle
        .modules
        .values()
        .map(|module| {
            (
                module.path.as_str().to_string(),
                pretty_codegen(module.program()),
            )
        })
        .collect();
    files.sort();
    Some(files)
}

fn split(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    files(unpack(&alloc, &parsed.program, &[]))
}

fn split_taken(source: &str) -> (Option<Vec<(String, String)>>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = files(unpack_taking(&alloc, &mut parsed.program, &[]));
    (found, pretty_codegen(&parsed.program))
}

fn code<'f>(files: &'f [(String, String)], path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, code)| code.as_str())
        .unwrap_or_else(|| panic!("no {path} in {files:?}"))
}

fn refused(source: &str) {
    assert_eq!(split(source), None, "split:\n{source}");
    let alloc = Allocator::default();
    let before = pretty_codegen(&parse_program(&alloc, source).program);
    let (taken, after) = split_taken(source);
    assert_eq!(taken, None);
    assert_eq!(after, before, "the refused input is left intact");
}

fn accepted(source: &str) -> Vec<(String, String)> {
    split(source).unwrap_or_else(|| panic!("no bundle:\n{source}"))
}

#[test]
fn a_chunk_with_one_runtime_entry_splits_into_commonjs_modules() {
    let files = accepted(&format!(
        "{}{}",
        push("1,(e,t,r)=>{var n=e.r(2);r.a=n.v;e.e.b=2},2,(e,t)=>{t.exports={v:1}},3,e=>{e.v(function(){return 3})}"),
        register("1")
    ));
    let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(paths, ["2.js", "3.js", "index.js"]);
    let entry = code(&files, "index.js");
    assert!(entry.contains("var n = require(\"./2.js\");"), "{entry}");
    assert!(entry.contains("exports.a = n.v;"), "{entry}");
    assert!(entry.contains("exports.b = 2;"), "{entry}");
    assert!(code(&files, "2.js").contains("module.exports = { v: 1 };"));
    assert!(code(&files, "3.js").contains("module.exports = function"));
}

#[test]
fn several_runtime_entries_get_a_synthesized_index_requiring_each_in_order() {
    let files = accepted(&format!(
        "{}{}",
        push("7,(e,t,r)=>{r.a=1},\"s\",(e,t,r)=>{r.b=e.r(7).a}"),
        register("\"s\",7")
    ));
    let entry = code(&files, "index.js");
    assert_eq!(entry, "require(\"./s.js\");\nrequire(\"./7.js\");");
    assert!(code(&files, "s.js").contains("require(\"./7.js\")"));
}

#[test]
fn a_chunk_without_a_registration_splits_with_no_entry() {
    let source = push("1,(e,t,r)=>{r.a=e.r(2).v},2,(e,t,r)=>{r.v=1}");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let result = unpack(&alloc, &parsed.program, &[]).expect("a bundle");
    assert_eq!(result.bundle.entry_id, None);
    let files = files(Some(result)).expect("files");
    assert!(code(&files, "1.js").contains("require(\"./2.js\")"));
}

#[test]
fn an_inner_binding_named_like_the_context_keeps_its_own_calls() {
    let files = accepted(&chunk(
        "(e,t,r)=>{r.a=function(e){return e.r(2)}({r:String});r.b=e.r(2).v}",
    ));
    let entry = code(&files, "index.js");
    assert!(entry.contains("return e.r(2);"), "{entry}");
    assert!(
        entry.contains("exports.b = require(\"./2.js\").v;"),
        "{entry}"
    );
}

#[test]
fn the_logical_assignment_holder_a_string_script_and_a_function_factory_are_accepted() {
    let source = format!(
        "(globalThis.TURBOPACK ||= []).push([typeof document == \"object\" ? document.currentScript : undefined, 1, function (e, module, exports) {{ \"use strict\"; exports.a = e.r(2).v; }}, {LEAF}]);\n\
         (globalThis.TURBOPACK ||= []).push([\"static/chunks/a.js\", {{ otherChunks: [], runtimeModuleIds: [1] }}]);\n"
    );
    let files = accepted(&source);
    let entry = code(&files, "index.js");
    assert!(entry.starts_with("\"use strict\";"), "{entry}");
    assert!(
        entry.contains("exports.a = require(\"./2.js\").v;"),
        "{entry}"
    );
}

#[test]
fn a_renamed_parameter_does_not_capture_a_host_name() {
    let files = accepted(&chunk("(e,t,r)=>{r.a=typeof window;t.exports.b=1}"));
    let entry = code(&files, "index.js");
    assert!(entry.contains("exports.a = typeof window;"), "{entry}");
    assert!(entry.contains("module.exports.b = 1;"), "{entry}");
}

#[test]
fn a_value_export_inside_a_statement_sequence_is_rewritten_in_place() {
    let files = accepted(&chunk("e=>{f(1),e.v(2),f(3)}"));
    let entry = code(&files, "index.js");
    assert!(entry.contains("module.exports = 2"), "{entry}");
    assert!(!entry.contains(".v("), "{entry}");
}

#[test]
fn the_fixture_extracts_alike_taken_and_cloned_and_loses_its_factory_bodies() {
    let source = fixture("min-entry/chunk.js");
    let cloned = split(&source);
    let (taken, after) = split_taken(&source);
    assert!(cloned.is_some());
    assert_eq!(taken, cloned);
    assert!(
        !after.contains("eval cycle-b"),
        "a factory body was cloned:\n{after}"
    );
}

#[test]
fn the_probe_sees_a_chunk_and_a_server_chunk_and_not_the_runtime() {
    for (name, want) in [
        ("min-entry/chunk.js", true),
        ("min-entry/runtime.js", false),
        (
            "server/server/chunks/ssr/node_modules__work_next_src_1k301osa1_9sg._.js",
            true,
        ),
    ] {
        let alloc = Allocator::default();
        let source = fixture(name);
        let parsed = parse_program(&alloc, &source);
        assert_eq!(
            untwist_unpack::holds_a_bundle(&parsed.program),
            want,
            "{name}"
        );
    }
}

#[test]
fn the_generated_chunks_and_the_esm_refusal_fixture_are_refused() {
    for name in [
        "browser-chunk.js",
        "neg-entry/chunk.js",
        "server/server/chunks/ssr/node_modules__work_next_src_1k301osa1_9sg._.js",
        "server/server/chunks/ssr/[externals]_path_1ulxq_vdi8_o7._.js",
    ] {
        refused(&fixture(name));
    }
}

#[test]
fn a_runtime_method_other_than_require_value_and_exports_refuses() {
    for body in [
        "var n=e.i(2)",
        "e.s([\"a\",0,1])",
        "e.n({})",
        "e.j({})",
        "e.A(2)",
        "e.l(\"x.js\")",
        "e.x(\"path\",()=>require(\"path\"))",
        "e.t(\"path\")",
        "e.g.a=1",
        "e.c",
        "e.m.exports=1",
    ] {
        refused(&chunk(&format!("e=>{{{body}}}")));
    }
}

#[test]
fn requiring_an_id_outside_the_chunk_or_of_the_other_literal_type_refuses() {
    refused(&chunk("e=>{e.r(9)}"));
    refused(&chunk("e=>{e.r(\"2\")}"));
    refused(&chunk("e=>{e.r(2,3)}"));
    refused(&chunk("e=>{e.r(n)}"));
}

#[test]
fn the_context_used_other_than_through_its_methods_refuses() {
    for body in [
        "f(e)",
        "e=1",
        "e.e=1",
        "delete e.e",
        "e.e()",
        "e.e``",
        "e?.r(2)",
        "e.r?.(2)",
        "e.v?.(1)",
        "f(e.v(1))",
        "e[\"r\"](2)",
        "var r=e.r;r(2)",
        "f(e?.e)",
    ] {
        refused(&chunk(&format!("e=>{{{body}}}")));
    }
}

#[test]
fn the_module_parameter_outside_exports_and_a_written_exports_parameter_refuse() {
    refused(&chunk("(e,t)=>{t.id}"));
    refused(&chunk("(e,t)=>{t={}}"));
    refused(&chunk("(e,t)=>{f(t)}"));
    refused(&chunk("(e,t)=>{f(t?.exports)}"));
    refused(&chunk("(e,t,r)=>{r={}}"));
}

#[test]
fn a_free_host_module_name_refuses() {
    for name in ["module", "exports", "require", "__filename", "__dirname"] {
        refused(&chunk(&format!("e=>{{f({name})}}")));
    }
}

#[test]
fn factory_level_arguments_eval_this_new_target_return_and_with_refuse() {
    refused(&chunk("e=>{f(arguments)}"));
    refused(&chunk("e=>{eval(\"1\")}"));
    refused(&chunk("e=>{f(this)}"));
    refused(&chunk("function(e){f(new.target)}"));
    refused(&chunk("function(e){return}"));
    refused(&chunk("function(e){with(o){}}"));
    refused(&chunk("e=>{f(import.meta)}"));
    accepted(&chunk(
        "e=>{f(function(){return [this,arguments,new.target]});f(()=>{return 1})}",
    ));
}

#[test]
fn factory_level_this_arguments_and_new_target_in_a_class_key_or_heritage_refuse() {
    refused(&chunk("e=>{class A extends f(this){}}"));
    refused(&chunk("e=>{class A{[this.k](){}}}"));
    refused(&chunk("e=>{class A{[arguments[0]]=1}}"));
    refused(&chunk("function(e){class A{static [new.target]=1}}"));
    accepted(&chunk(
        "e=>{class A{m(){return this}x=this;static{f(this)}}}",
    ));
}

#[test]
fn a_binding_named_require_module_or_exports_refuses() {
    for body in [
        "var require=1",
        "function module(){}",
        "f(function(exports){})",
    ] {
        refused(&chunk(&format!("e=>{{{body}}}")));
    }
}

#[test]
fn parameters_outside_the_three_simple_canonical_ones_refuse() {
    for params in [
        "e,t,r,n",
        "...e",
        "e=1",
        "{e}",
        "e,e",
        "e,exports",
        "module",
        "e,t,require",
    ] {
        refused(&chunk(&format!("({params})=>{{}}")));
    }
    refused(&chunk("(e,t)=>{var t}"));
}

#[test]
fn a_factory_that_is_async_a_generator_named_or_expression_bodied_refuses() {
    for factory in [
        "async e=>{}",
        "function*(e){}",
        "async function(e){}",
        "function f(e){}",
        "e=>1",
    ] {
        refused(&chunk(factory));
    }
}

#[test]
fn a_chunk_layout_the_runtime_would_read_otherwise_refuses() {
    refused(&push("1,3,(e,t,r)=>{r.a=1}"));
    refused(&push("1,(e,t,r)=>{r.a=1},2"));
    refused(&push("[1,e=>{}],2,e=>{}"));
    refused(&push("1,e=>{},1,e=>{}"));
    refused(&push("1,e=>{},\"1\",e=>{}"));
    refused(&push("e=>{}"));
    refused(&format!("\"use strict\";{}", push(LEAF)));
    refused(&format!("{}f();\n", push(LEAF)));
    refused(&format!("{HOLDER}([document.currentScript,{LEAF}]);\n"));
    refused(&format!("{HOLDER}([{SCRIPT},{LEAF}],1);\n"));
    refused(&format!("{HOLDER}([{SCRIPT},...a,{LEAF}]);\n"));
    refused(&format!(
        "(self.TURBOPACK||(self.TURBOPACK=[])).push([{SCRIPT},{LEAF}]);\n"
    ));
}

#[test]
fn a_registration_other_than_one_with_no_other_chunks_and_known_ids_refuses() {
    refused(&format!(
        "{}{}",
        push(LEAF),
        push("{otherChunks:[\"a.js\"],runtimeModuleIds:[2]}")
    ));
    refused(&format!("{}{}", push(LEAF), register("3")));
    refused(&format!("{}{}", push(LEAF), register("\"2\"")));
    refused(&format!("{}{}{}", push(LEAF), register("2"), register("2")));
    refused(&format!(
        "{}{}",
        push(LEAF),
        push("{otherChunks:[],runtimeModuleIds:[2],x:[]}")
    ));
    refused(&format!("{}{}", push(LEAF), push("{runtimeModuleIds:[2]}")));
    refused(&format!(
        "{}{}",
        push("\"index\",e=>{},2,e=>{}"),
        register("\"index\",2")
    ));
}
