use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::unpack;

const HELPERS: &str = "var __getOwnPropNames = Object.getOwnPropertyNames;
var __require = ((x) => typeof require !== \"undefined\" ? require : typeof Proxy !== \"undefined\" ? new Proxy(x, {
  get: (a, b) => (typeof require !== \"undefined\" ? require : a)[b]
}) : x)(function(x) {
  if (typeof require !== \"undefined\") return require.apply(this, arguments);
  throw Error('Dynamic require of \"' + x + '\" is not supported');
});
var __commonJS = (cb, mod) => function __require2() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
";

const BANNER: &str =
    "import { createRequire } from \"module\";\nconst require = createRequire(import.meta.url);\n";

const FACTORY: &str = "var require_a = __commonJS({
  \"a.js\"(exports, module) {
    module.exports = typeof __require(\"fs\").readFileSync;
  }
});
";

fn split(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let result = unpack(&alloc, &parsed.program, &[])?;
    let module = result
        .bundle
        .modules
        .values()
        .find(|module| module.path.as_str() == "a.js")?;
    Some(pretty_codegen(module.program()))
}

#[test]
fn an_esm_entry_without_a_created_require_keeps_the_throwing_shim() {
    let source = format!(
        "import {{ format }} from \"node:util\";\n{HELPERS}{FACTORY}console.log(format(\"%s\", require_a()));\n"
    );
    assert_eq!(split(&source), None);
}

#[test]
fn an_esm_entry_whose_shim_call_stands_in_the_entry_is_not_split() {
    let source = format!(
        "export const fs = 1;\n{HELPERS}{FACTORY}console.log(require_a(), __require(\"path\"));\n"
    );
    assert_eq!(split(&source), None);
}

#[test]
fn an_esm_entry_with_the_created_require_banner_still_splits() {
    let source = format!("{BANNER}{HELPERS}{FACTORY}console.log(require_a());\n");
    let module = split(&source).expect("split");
    assert!(module.contains("require(\"fs\")"), "{module}");
}

#[test]
fn a_commonjs_bundle_still_turns_the_shim_into_require() {
    let source = format!("{HELPERS}{FACTORY}console.log(require_a());\n");
    let module = split(&source).expect("split");
    assert!(module.contains("require(\"fs\")"), "{module}");
}
