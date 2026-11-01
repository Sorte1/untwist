use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking, BundleKind, UnpackResult};

const GLOBAL: &str = "parcelRequire94c2";
const LEAF: &str = "b:[function(e,r,t){t.v=1},{}]";
const REGISTER: &str = "parcelRegister(\"b1\", function(module, exports) {\nexports.v = 1;\n});\n";
const USE: &str = "console.log(parcelRequire(\"b1\").v);\n";

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/unpack-bundlers/parcel")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

fn runtime() -> String {
    let text = fixture("min-parcel-nohoist.js");
    let end = text
        .find("}({")
        .expect("the runtime ends before the module map");
    text[..=end].to_string()
}

fn prelude() -> String {
    let text = fixture("min-parcel-hoist.js");
    let start = text.find("function $parcel$export").expect("prelude start");
    let marker = "var parcelRegister = parcelRequire.register;\n";
    let end = text.find(marker).expect("prelude end") + marker.len();
    text[start..end].to_string()
}

fn registry_with(modules: &str, tail: &str) -> String {
    format!("{}({{{modules}}},{tail});\n", runtime())
}

fn registry(modules: &str) -> String {
    registry_with(modules, &format!("[\"a\"],\"a\",\"{GLOBAL}\",{{}}"))
}

fn entry(params: &str, body: &str, deps: &str) -> String {
    format!("a:[function({params}){{{body}}},{{{deps}}}],{LEAF}")
}

fn hoisted(before: &str, registers: &str, remainder: &str) -> String {
    format!("{before}{}{registers}{remainder}", prelude())
}

fn files(result: Option<UnpackResult<'_>>) -> Option<Vec<(String, String)>> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Parcel);
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
        .unwrap_or_else(|| panic!("no {path} in {:?}", paths(files)))
}

fn paths(files: &[(String, String)]) -> Vec<&str> {
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

fn accepted(source: &str) -> Vec<(String, String)> {
    split(source).unwrap_or_else(|| panic!("no bundle:\n{source}"))
}

#[test]
fn a_registry_bundle_splits_and_its_requires_name_paths() {
    let files = accepted(&fixture("min-parcel-nohoist.js"));
    assert_eq!(
        paths(&files),
        ["b.js", "c.js", "e.js", "index.js", "j.js", "x.js", "y.js"]
    );
    let index = code(&files, "index.js");
    assert!(
        index.contains(
            "var b = require(\"./b.js\"), c = require(\"./c.js\"), path = require(\"path\");"
        ),
        "{index}"
    );
    assert!(
        index.contains("Promise.resolve(require(\"./e.js\"))"),
        "{index}"
    );
    assert!(
        index.starts_with("var __globalThis = globalThis;"),
        "{index}"
    );
    let b = code(&files, "b.js");
    assert!(
        b.starts_with(
            "var __globalThis = globalThis;\nconsole.log(\"eval b\");\nexports.name = \"bee\";"
        ),
        "{b}"
    );
    let c = code(&files, "c.js");
    assert!(
        c.contains("var j = require(\"./j.js\"), b = require(\"./b.js\");"),
        "{c}"
    );
    assert!(c.contains("module.exports = {"), "{c}");
}

#[test]
fn several_entries_get_a_synthesized_index_that_exports_the_main_entry() {
    let files = accepted(&fixture("min-parcel-entries.js"));
    assert_eq!(paths(&files), ["a.js", "index.js", "m.js", "s.js"]);
    assert_eq!(
        code(&files, "index.js"),
        "require(\"./a.js\");\nrequire(\"./m.js\");\nmodule.exports = require(\"./m.js\");"
    );
}

#[test]
fn a_hoisted_bundle_moves_its_registered_factories_and_the_helpers_they_read() {
    let files = accepted(&fixture("min-parcel-hoist.js"));
    assert_eq!(
        paths(&files),
        ["_runtime/parcel.js", "d4t4A.js", "index.js", "l4t3B.js"]
    );
    let index = code(&files, "index.js");
    assert!(index.starts_with("var { $parcel$export } = require(\"./_runtime/parcel.js\");\nvar $ext$path = require(\"path\");"), "{index}");
    assert!(!index.contains("parcelRegister(\""), "{index}");
    assert!(!index.contains("function $parcel$export"), "{index}");
    assert!(
        index.contains("var $d4t4A = require(\"./d4t4A.js\");"),
        "{index}"
    );
    assert!(
        index.contains("return require(\"./l4t3B.js\").value;"),
        "{index}"
    );
    let late = code(&files, "l4t3B.js");
    assert!(
        late.starts_with("var { $parcel$export } = require(\"./_runtime/parcel.js\");"),
        "{late}"
    );
    assert!(
        late.contains(
            "require(\"path\").sep.length + \"-\" + require(\"./d4t4A.js\").items.length"
        ),
        "{late}"
    );
    let runtime = code(&files, "_runtime/parcel.js");
    assert!(
        runtime
            .trim_end()
            .ends_with("module.exports = { $parcel$export };"),
        "{runtime}"
    );
}

#[test]
fn minified_registers_in_a_sequence_split_and_their_parameters_are_renamed() {
    let files = accepted(&fixture("min-parcel-hoist-seq.js"));
    assert_eq!(
        paths(&files),
        ["_runtime/parcel.js", "index.js", "q1.js", "q2.js"]
    );
    let q1 = code(&files, "q1.js");
    assert!(
        q1.contains("$parcel$export(module.exports, \"value\""),
        "{q1}"
    );
    let q2 = code(&files, "q2.js");
    assert!(
        q2.contains("exports.join = function(e, a)")
            || q2.contains("exports.join = function (e, a)"),
        "{q2}"
    );
    assert!(
        q2.contains("exports.of = require(\"./q1.js\").value"),
        "{q2}"
    );
    let index = code(&files, "index.js");
    assert!(
        index.contains("$s$say(\"eval index\"), $a$x = require(\"./q2.js\")"),
        "{index}"
    );
}

#[test]
fn a_renamed_parameter_does_not_capture_a_root_binding_of_the_canonical_name() {
    let files = accepted(&registry(&entry(
        "e,r,t",
        "var module=1;r.exports={m:module,v:e(\"./b.js\").v}",
        "\"./b.js\":\"b\"",
    )));
    let index = code(&files, "index.js");
    assert!(index.contains("var _module = 1;"), "{index}");
    assert!(
        index.contains("module.exports = {\n  m: _module,\n  v: require(\"./b.js\").v\n};"),
        "{index}"
    );
}

#[test]
fn an_inner_require_binding_keeps_its_own_calls() {
    let files = accepted(&registry(&entry(
        "e,r,t",
        "function g(require){return require(\"./b.js\")}t.g=g;t.v=e(\"./b.js\").v",
        "\"./b.js\":\"b\"",
    )));
    let index = code(&files, "index.js");
    assert!(
        index.contains("function g(require) {\n  return require(\"./b.js\");\n}"),
        "{index}"
    );
    assert!(
        index.contains("exports.v = require(\"./b.js\").v;"),
        "{index}"
    );
}

#[test]
fn an_undeclared_spec_names_a_module_by_its_id() {
    let files = accepted(&registry(&entry("e,r,t", "t.v=e(\"b\").v", "")));
    assert!(code(&files, "index.js").contains("exports.v = require(\"./b.js\").v;"));
}

#[test]
fn the_fixtures_extract_alike_taken_and_cloned_and_lose_their_factory_bodies() {
    for name in [
        "min-parcel-nohoist.js",
        "min-parcel-entries.js",
        "min-parcel-hoist.js",
        "min-parcel-hoist-seq.js",
        "single-hoist.js",
        "single-hoist-min.js",
        "single-nohoist.js",
        "single-nohoist-min.js",
    ] {
        let source = fixture(name);
        let cloned = split(&source);
        let (taken, after) = split_taken(&source);
        assert!(cloned.is_some(), "{name}");
        assert_eq!(taken, cloned, "{name}");
        for probe in ["\"eval b\"", "eval late", "eval q1", "\"eval s\""] {
            assert!(
                !(source.contains(probe) && after.contains(probe)),
                "{name}: a factory body was cloned:\n{after}"
            );
        }
    }
}

#[test]
fn the_generated_builds_load_a_second_bundle_and_are_refused() {
    for name in [
        "hoist/index.js",
        "hoist/lazy.58e54a87.js",
        "nohoist/index.js",
        "nohoist/lazy.99d34c29.js",
    ] {
        refused(&fixture(name));
    }
}

#[test]
fn loading_another_bundle_file_refuses() {
    refused(&fixture("neg-parcel-chunk-load.js"));
    refused(&registry(&entry(
        "e,r,t",
        "t.l=function(){return r.require(\"./c.js\")}",
        "",
    )));
}

#[test]
fn a_dependency_outside_the_bundle_that_is_not_a_builtin_refuses() {
    refused(&fixture("neg-parcel-foreign-id.js"));
    refused(&registry(&entry("e,r,t", "e(\"x\")", "\"x\":\"lodash\"")));
    assert!(split(&registry(&entry("e,r,t", "e(\"x\")", "\"x\":\"node:fs\""))).is_some());
}

#[test]
fn a_dependency_map_value_that_is_not_a_string_refuses() {
    refused(&registry(&entry("e,r,t", "e(\"x\")", "\"x\":false")));
    refused(&registry(&entry(
        "e,r,t",
        "e(\"x\")",
        "\"x\":[[\"a\",\"b\"]]",
    )));
}

#[test]
fn a_require_argument_that_is_not_one_string_refuses() {
    refused(&registry(&entry(
        "e,r,t",
        "var x=\"./b.js\";e(x)",
        "\"./b.js\":\"b\"",
    )));
    refused(&registry(&entry(
        "e,r,t",
        "e(\"./b.js\",1)",
        "\"./b.js\":\"b\"",
    )));
}

#[test]
fn the_require_parameter_used_as_a_value_refuses() {
    refused(&registry(&entry("e,r,t", "t.r=e", "")));
    refused(&registry(&entry("e,r,t", "e.resolve(\"./b.js\")", "")));
}

#[test]
fn the_module_parameter_outside_exports_typeof_and_bundle_root_refuses() {
    refused(&registry(&entry("e,r,t", "t.id=r.id", "")));
    refused(&registry(&entry("e,r,t", "t.m=r", "")));
    refused(&registry(&entry("e,r,t", "t.p=r.bundle.root(\"zz\")", "")));
}

#[test]
fn the_factory_arguments_and_direct_eval_refuse() {
    refused(&registry(&entry("e,r,t", "t.n=arguments.length", "")));
    refused(&registry(&entry("e,r,t", "eval(\"1\")", "")));
}

#[test]
fn a_host_name_read_free_in_a_factory_refuses() {
    refused(&registry(&entry("e,r,t", "t.m=module", "")));
    refused(&registry(&entry("e,r,t", "t.f=__filename", "")));
    refused(&registry(&entry("e,r,t", "t.e=exports", "")));
}

#[test]
fn a_rewritten_require_under_a_require_binding_refuses() {
    refused(&registry(&entry(
        "e,r,t",
        "function g(require){return e(\"./b.js\")}t.g=g",
        "\"./b.js\":\"b\"",
    )));
}

#[test]
fn a_parameter_redeclared_at_the_factory_root_refuses() {
    refused(&registry(&entry("e,r,t", "var t;t.v=1", "")));
    refused(&registry(&entry("e,r,t,o", "function o(){}t.v=o", "")));
}

#[test]
fn a_canonical_name_in_another_position_refuses() {
    refused(&registry(&entry("module,require", "require.exports=1", "")));
    refused(&registry(&entry("e,e", "e.v=1", "")));
}

#[test]
fn an_inherited_name_as_a_spec_or_an_id_refuses() {
    refused(&registry(&entry("e,r,t", "e(\"toString\")", "")));
    refused(&registry(&format!(
        "{},constructor:[function(){{}},{{}}]",
        entry("e,r,t", "", "")
    )));
}

#[test]
fn the_runtime_arguments_must_be_inert() {
    let modules = entry("e,r,t", "t.v=1", "");
    refused(&registry_with(
        &modules,
        &format!("[\"a\"],\"a\",\"{GLOBAL}\",{{path:1}}"),
    ));
    refused(&registry_with(
        &modules,
        &format!("[\"a\"],\"a\",\"{GLOBAL}\",{{}},f()"),
    ));
}

#[test]
fn entries_must_be_modules_of_the_bundle() {
    let modules = entry("e,r,t", "t.v=1", "");
    refused(&registry_with(
        &modules,
        &format!("[\"zz\"],\"a\",\"{GLOBAL}\",{{}}"),
    ));
    refused(&registry_with(
        &modules,
        &format!("[\"a\"],\"zz\",\"{GLOBAL}\",{{}}"),
    ));
    let index = format!("{modules},index:[function(){{}},{{}}]");
    refused(&registry_with(
        &index,
        &format!("[\"a\",\"b\"],null,\"{GLOBAL}\",{{}}"),
    ));
}

#[test]
fn a_top_level_return_new_target_or_with_refuses() {
    refused(&registry(&entry("e,r,t", "if(t)return;t.v=1", "")));
    refused(&registry(&entry("e,r,t", "t.v=new.target", "")));
    refused(&registry(&entry("e,r,t", "with(t){v=1}", "")));
    assert!(split(&registry(&entry("e,r,t", "t.f=()=>{return 1}", ""))).is_some());
}

#[test]
fn a_free_read_of_the_registry_global_refuses() {
    refused(&registry(&entry("e,r,t", &format!("t.r={GLOBAL}"), "")));
}

#[test]
fn a_strict_program_or_runtime_refuses() {
    refused(&format!(
        "\"use strict\";\n{}",
        registry(&entry("e,r,t", "t.v=1", ""))
    ));
}

#[test]
fn a_global_parameter_with_a_root_global_this_binding_refuses() {
    refused(&registry(&entry("e,r,t,o", "var globalThis=1;t.v=o", "")));
}

#[test]
fn a_relative_host_require_refuses() {
    refused(&registry(&entry("e,r,t", "t.x=require(\"./x.js\")", "")));
    assert!(split(&registry(&entry("e,r,t", "t.x=require(\"path\")", ""))).is_some());
}

#[test]
fn a_second_top_level_statement_refuses() {
    refused(&format!(
        "{}console.log(1);\n",
        registry(&entry("e,r,t", "t.v=1", ""))
    ));
}

#[test]
fn a_runtime_other_than_parcels_refuses() {
    let modules = entry("e,r,t", "t.v=1", "");
    let tail = format!("[\"a\"],\"a\",\"{GLOBAL}\",{{}}");
    let runtime = runtime();
    let with = |runtime: &str| format!("{runtime}({{{modules}}},{tail});\n");
    assert!(split(&with(&runtime)).is_some());
    let effect = runtime.replacen("{var ", "{console.log(1);var ", 1);
    refused(&with(&effect));
    refused(&with("!function(e,r,t,o,n){console.log(1)}"));
    refused(&with(&runtime.replacen("Error(", "TypeError(", 1)));
    refused(&with(&runtime.replacen(
        "return u[r].exports",
        "return l[r].exports",
        1,
    )));
    refused(&with(&runtime.replacen(
        "module.exports=p",
        "exports.exports=p",
        1,
    )));
    let source = fixture("single-nohoist.js");
    assert!(split(&source).is_some());
    refused(&source.replacen("var cache = ", "console.log(1);\n  var cache = ", 1));
}

#[test]
fn a_sequence_around_the_registry_call_refuses() {
    let source = registry(&entry("e,r,t", "t.v=1", ""));
    let call = source.trim_end().trim_end_matches(';');
    assert!(split(&source).is_some());
    refused(&format!("f(),{call};\n"));
    refused(&format!("!(f(),{});\n", call.trim_start_matches('!')));
}

#[test]
fn a_registered_factory_that_reads_a_hoisted_binding_refuses() {
    refused(&fixture("neg-parcel-bundle-binding.js"));
}

#[test]
fn a_register_after_an_effect_refuses() {
    refused(&fixture("neg-parcel-late-register.js"));
    refused(&hoisted(
        "",
        &format!(
            "console.log(0), {}",
            REGISTER.trim_end().trim_end_matches(';')
        ),
        ";\n",
    ));
}

#[test]
fn a_registry_call_of_an_unregistered_id_refuses() {
    refused(&hoisted(
        "",
        REGISTER,
        "console.log(parcelRequire(\"zz\"));\n",
    ));
    let register = "parcelRegister(\"b1\", function(module, exports) {\nexports.v = parcelRequire(\"zz\");\n});\n";
    refused(&hoisted("", register, USE));
}

#[test]
fn the_registry_used_as_a_value_refuses() {
    refused(&hoisted(
        "",
        REGISTER,
        "var r = parcelRequire;\nconsole.log(r(\"b1\"));\n",
    ));
    refused(&hoisted("", REGISTER, "parcelRequire = null;\n"));
}

#[test]
fn the_register_binding_used_elsewhere_refuses() {
    let nested = "if (true) parcelRegister(\"c1\", function(module, exports) {});\n";
    refused(&hoisted("", REGISTER, &format!("{USE}{nested}")));
}

#[test]
fn a_moved_helper_written_by_the_entry_refuses() {
    let register = "parcelRegister(\"b1\", function(module, exports) {\n$parcel$export(exports, \"v\", () => 1);\n});\n";
    refused(&hoisted(
        "",
        register,
        &format!("{USE}$parcel$export = null;\n"),
    ));
    assert!(split(&hoisted("", register, USE)).is_some());
}

#[test]
fn a_helper_that_is_not_closed_refuses() {
    let register =
        "parcelRegister(\"b1\", function(module, exports) {\nexports.v = $help();\n});\n";
    let helper = "function $help() {\n    return $top;\n}\nvar $top = 1;\n";
    refused(&hoisted(helper, register, USE));
    let closed = "function $help() {\n    return 1;\n}\n";
    assert!(split(&hoisted(closed, register, USE)).is_some());
}

#[test]
fn a_relative_host_require_in_a_factory_or_the_entry_refuses() {
    let register = "parcelRegister(\"b1\", function(module, exports) {\nexports.v = require(\"./x.js\");\n});\n";
    refused(&hoisted("", register, USE));
    refused(&hoisted(
        "",
        REGISTER,
        &format!("{USE}require(\"./x.js\");\n"),
    ));
}

#[test]
fn a_chunk_without_the_registry_definition_refuses() {
    refused(&fixture("neg-parcel-no-registry.js"));
}

#[test]
fn a_binding_named_like_the_registry_refuses() {
    let register = "parcelRegister(\"b1\", function(module, exports) {\nvar parcelRequire = 1;\nexports.v = parcelRequire;\n});\n";
    refused(&hoisted("", register, USE));
}

#[test]
fn the_module_parameter_outside_exports_and_typeof_refuses() {
    let register =
        "parcelRegister(\"b1\", function(module, exports) {\nexports.v = module.id;\n});\n";
    refused(&hoisted("", register, USE));
}

#[test]
fn an_entry_read_of_the_host_wrapper_refuses() {
    refused(&hoisted(
        "",
        REGISTER,
        &format!("{USE}console.log(__filename);\n"),
    ));
    refused(&hoisted(
        "",
        REGISTER,
        &format!("{USE}console.log(arguments.length);\n"),
    ));
}

#[test]
fn duplicate_register_ids_refuse() {
    refused(&hoisted("", &format!("{REGISTER}{REGISTER}"), USE));
}

#[test]
fn a_moved_helper_that_reads_the_host_or_top_level_this_refuses() {
    let register =
        "parcelRegister(\"b1\", function(module, exports) {\nexports.v = $help();\n});\n";
    refused(&hoisted(
        "function $help() {\n    return module;\n}\n",
        register,
        USE,
    ));
    refused(&hoisted("var $help = () => this;\n", register, USE));
    refused(&hoisted(
        "var $help = () => class { [this]() {} };\n",
        register,
        USE,
    ));
}

#[test]
fn module_syntax_refuses() {
    refused(&hoisted("import \"path\";\n", REGISTER, USE));
}

#[test]
fn a_factory_that_reads_the_register_binding_refuses() {
    let register =
        "parcelRegister(\"b1\", function(module, exports) {\nexports.v = parcelRegister;\n});\n";
    refused(&hoisted("", register, USE));
}

#[test]
fn a_require_parameter_shadowed_inside_keeps_the_inner_calls() {
    let files = accepted(&registry(&entry(
        "require,module,exports",
        "function g(require){return require(\"B\")}exports.g=g;exports.v=require(\"B\").v",
        "\"B\":\"b\"",
    )));
    let index = code(&files, "index.js");
    assert!(index.contains("return require(\"B\");"), "{index}");
    assert!(
        index.contains("exports.v = require(\"./b.js\").v;"),
        "{index}"
    );
}

#[test]
fn a_factory_outside_the_simple_shape_refuses() {
    refused(&registry(&entry("e,r,t,o,n", "t.v=1", "")));
    refused(&registry(&entry("e,r,t=1", "t.v=1", "")));
    refused(&registry(&format!(
        "a:[async function(e,r,t){{t.v=1}},{{}}],{LEAF}"
    )));
}

#[test]
fn a_prelude_without_the_registry_definition_refuses() {
    let full = prelude();
    let start = full.find("if (parcelRequire == null)").expect("definition");
    let end = full.find("var parcelRegister").expect("register binding");
    let chunk = format!("{}{}", &full[..start], &full[end..]);
    let source = format!("{chunk}{REGISTER}{USE}");
    refused(&source);
}

#[test]
fn a_module_id_that_names_the_entry_or_the_runtime_refuses() {
    let register = |id: &str| REGISTER.replace("\"b1\"", &format!("\"{id}\""));
    let using = |id: &str| USE.replace("\"b1\"", &format!("\"{id}\""));
    assert!(split(&hoisted("", &register("b1"), &using("b1"))).is_some());
    refused(&hoisted("", &register("index"), &using("index")));
    refused(&hoisted(
        "",
        &register("_runtime/parcel"),
        &using("_runtime/parcel"),
    ));
    assert!(split(&registry(&entry(
        "e,r,t",
        "t.v=e(\"./b\").v",
        "\"./b\":\"b\""
    )))
    .is_some());
    refused(&registry(&format!(
        "{},\"_runtime/parcel\":[function(e,r,t){{t.v=1}},{{}}]",
        entry("e,r,t", "t.v=e(\"./b\").v", "\"./b\":\"_runtime/parcel\"")
    )));
}

const STRESS_MODULES: usize = 100_000;
const STRESS_SECONDS: u64 = 30;

fn stress_registers(separator: &str) -> String {
    let registers: Vec<String> = (0..STRESS_MODULES)
        .map(|i| {
            format!("parcelRegister(\"m{i}\", function(module, exports) {{\nexports.v = {i};\n}})")
        })
        .collect();
    format!("{};\n", registers.join(separator))
}

#[test]
fn a_hoisted_bundle_of_many_modules_splits_in_linear_time() {
    let tail = "console.log(parcelRequire(\"m0\").v);\n";
    for separator in [";\n", ",\n"] {
        let source = hoisted("", &stress_registers(separator), tail);
        let started = std::time::Instant::now();
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        let result = unpack(&alloc, &parsed.program, &[]).expect("a parcel bundle");
        assert_eq!(result.bundle.modules.len(), STRESS_MODULES + 1);
        let elapsed = started.elapsed();
        assert!(elapsed.as_secs() < STRESS_SECONDS, "{elapsed:?}");
    }
}

#[test]
fn a_dependency_map_with_a_repeated_or_prototype_key_refuses() {
    let other = "c:[function(e,r,t){t.v=2},{}]";
    let with = |deps: &str| {
        registry(&format!(
            "{},{other}",
            entry("e,r,t", "t.v=e(\"./b\").v", deps)
        ))
    };
    accepted(&with("\"./b\":\"b\",\"./c\":\"c\""));
    let unordered = accepted(&with("\"./z\":\"c\",\"./y\":\"c\",\"./b\":\"b\""));
    assert!(
        code(&unordered, "index.js").contains("require(\"./b.js\")"),
        "{unordered:?}"
    );
    refused(&with("\"./b\":\"b\",\"./c\":\"c\",\"./b\":\"c\""));
    refused(&with("\"./b\":\"b\",\"./b\":\"c\""));
    refused(&with("\"./b\":\"c\",\"./b\":\"b\""));
    refused(&with("\"./b\":\"b\",\"__proto__\":\"c\""));
    refused(&with("\"1\":\"b\",1:\"c\""));
}

const STRESS_DEPS: usize = 100_000;

#[test]
fn one_module_with_many_dependencies_links_in_linear_time() {
    let deps: Vec<String> = (0..STRESS_DEPS)
        .map(|i| format!("\"./b{i}\":\"b\""))
        .collect();
    let body: Vec<String> = (0..STRESS_DEPS)
        .map(|i| format!("t.v{i}=e(\"./b{i}\").v"))
        .collect();
    let source = registry(&entry("e,r,t", &body.join(";"), &deps.join(",")));
    let started = std::time::Instant::now();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let result = unpack(&alloc, &parsed.program, &[]).expect("a parcel bundle");
    assert_eq!(result.bundle.modules.len(), 2);
    let elapsed = started.elapsed();
    assert!(elapsed.as_secs() < STRESS_SECONDS, "{elapsed:?}");
}

#[test]
fn a_require_of_an_object_prototype_key_refuses() {
    let with = |body: &str, deps: &str| registry(&entry("e,r,t", body, deps));
    let own = accepted(&with("t.v=e(\"constructor\").v", "\"constructor\":\"b\""));
    assert!(
        code(&own, "index.js").contains("require(\"./b.js\")"),
        "{own:?}"
    );
    for key in [
        "constructor",
        "toString",
        "valueOf",
        "hasOwnProperty",
        "__proto__",
        "__lookupGetter__",
    ] {
        refused(&with(&format!("t.v=e(\"{key}\")"), "\"./b\":\"b\""));
        refused(&with(&format!("t.v=e(\"{key}\")"), ""));
    }
    let named = |id: &str| {
        registry(&format!(
            "{},{id}:[function(e,r,t){{t.v=2}},{{}}]",
            entry("e,r,t", &format!("t.v=e(\"{id}\").v"), "")
        ))
    };
    accepted(&named("c"));
    refused(&named("constructor"));
    refused(&named("toString"));
    let register = |id: &str| REGISTER.replace("\"b1\"", &format!("\"{id}\""));
    let using = |id: &str| USE.replace("\"b1\"", &format!("\"{id}\""));
    assert!(split(&hoisted("", &register("b1"), &using("b1"))).is_some());
    refused(&hoisted("", REGISTER, &using("constructor")));
    refused(&hoisted("", REGISTER, &using("toString")));
    refused(&hoisted(
        "",
        &register("constructor"),
        &using("constructor"),
    ));
    refused(&hoisted("", &register("valueOf"), &using("valueOf")));
}
