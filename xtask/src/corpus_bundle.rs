use std::path::Path;

use anyhow::{bail, Context, Result};

pub const OUTPUT: &str = "fixtures/generated/webpack-5mb.js";
pub const DEFAULT_MODULES: usize = 1150;
pub const DEFAULT_SEED: u64 = 0x5eed_4b12;
pub const KINDS: [&str; 5] = ["webpack", "esbuild", "metro", "systemjs", "amd"];

const HEADER: &str = "(function (modules) {\n  var cache = {};\n  function __webpack_require__(id) {\n    if (cache[id]) return cache[id].exports;\n    var module = cache[id] = { exports: {} };\n    modules[id].call(module.exports, module, module.exports, __webpack_require__);\n    return module.exports;\n  }\n  return __webpack_require__(0);\n})([\n";

const ESBUILD_HEADER: &str = "var __getOwnPropNames = Object.getOwnPropertyNames;\nvar __commonJS = (cb, mod) => function __require() {\n  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;\n};\n";

const METRO_HEADER: &str = "var modules = Object.create(null);\nfunction __d(factory, moduleId, dependencyMap) {\n  modules[moduleId] = { factory: factory, dependencyMap: dependencyMap, module: null };\n}\nfunction __r(moduleId) {\n  var record = modules[moduleId];\n  if (record.module) return record.module.exports;\n  var module = (record.module = { exports: {} });\n  record.factory(globalThis, __r, null, null, module, module.exports, record.dependencyMap);\n  return module.exports;\n}\n";

const FACTORY_PARAMS_METRO: &str =
    "global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap";

pub fn output_for(kind: &str) -> String {
    if kind == "webpack" {
        OUTPUT.to_string()
    } else {
        format!("fixtures/generated/kinds/{kind}-5mb.js")
    }
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn body_lines(kind: &str, index: usize, state: &mut u64, indent: &str) -> String {
    let mut body = String::with_capacity(4096);
    if kind == "systemjs" {
        body.push_str(&format!("{indent}_export(\"id\", {index});\n"));
    }
    for line in 0..60 {
        let value = next(state) % 100_000;
        let export = if kind == "systemjs" {
            format!("_export(\"k{line:02}\", function (a, b) {{ return a + b + v{line}; }});")
        } else {
            format!("exports.k{line} = function (a, b) {{ return a + b + v{line}; }};")
        };
        body.push_str(&format!(
            "{indent}var v{line} = {value};\n{indent}{export}\n"
        ));
    }
    if kind != "systemjs" {
        body.push_str(&format!("{indent}exports.id = {index};\n"));
    }
    body
}

fn webpack_module(index: usize, state: &mut u64) -> String {
    let mut text =
        String::from("function (module, exports, __webpack_require__) {\n  \"use strict\";\n");
    if index > 0 {
        text.push_str(&format!(
            "  var dep = __webpack_require__({});\n",
            index - 1
        ));
    }
    text.push_str(&body_lines("webpack", index, state, "  "));
    text.push('}');
    text
}

fn esbuild_module(index: usize, state: &mut u64) -> String {
    let mut text = format!("var require_m{index} = __commonJS({{\n  \"src/m{index}.js\"(exports, module) {{\n    \"use strict\";\n");
    if index > 0 {
        text.push_str(&format!("    var dep = require_m{}();\n", index - 1));
    }
    text.push_str(&body_lines("esbuild", index, state, "    "));
    text.push_str("  }\n});\n");
    text
}

fn metro_module(index: usize, state: &mut u64) -> String {
    let mut text = format!("__d(function ({FACTORY_PARAMS_METRO}) {{\n  \"use strict\";\n");
    let deps = if index > 0 {
        text.push_str("  var dep = require(_dependencyMap[0]);\n");
        format!("[{}]", index - 1)
    } else {
        "[]".to_string()
    };
    text.push_str(&body_lines("metro", index, state, "  "));
    text.push_str(&format!("}}, {index}, {deps});\n"));
    text
}

fn systemjs_module(index: usize, state: &mut u64) -> String {
    let (deps, setters) = if index > 0 {
        (
            format!("[\"m{}\"]", index - 1),
            "[function (m) { dep = m; }]",
        )
    } else {
        ("[]".to_string(), "[]")
    };
    let mut text = format!("System.register(\"m{index}\", {deps}, function (_export, _context) {{\n  \"use strict\";\n  var dep;\n  return {{\n    setters: {setters},\n    execute: function () {{\n");
    text.push_str(&body_lines("systemjs", index, state, "      "));
    text.push_str("    }\n  };\n});\n");
    text
}

fn amd_module(index: usize, state: &mut u64) -> String {
    let (deps, params) = if index > 0 {
        (format!("[\"exports\", \"m{}\"]", index - 1), "exports, dep")
    } else {
        ("[\"exports\"]".to_string(), "exports")
    };
    let mut text =
        format!("define(\"m{index}\", {deps}, function ({params}) {{\n  \"use strict\";\n");
    text.push_str(&body_lines("amd", index, state, "  "));
    text.push_str("});\n");
    text
}

fn webpack_text(modules: usize, state: &mut u64) -> String {
    let mut text = String::from(HEADER);
    for index in 0..modules {
        if index > 0 {
            text.push_str(",\n");
        }
        text.push_str(&webpack_module(index, state));
    }
    text.push_str("\n]);\n");
    text
}

fn listed_text(kind: &str, modules: usize, state: &mut u64) -> Result<String> {
    let (header, module, footer): (&str, fn(usize, &mut u64) -> String, String) = match kind {
        "esbuild" => (
            ESBUILD_HEADER,
            esbuild_module,
            format!("require_m{}();\n", modules - 1),
        ),
        "metro" => (
            METRO_HEADER,
            metro_module,
            format!("__r({});\n", modules - 1),
        ),
        "systemjs" => ("", systemjs_module, String::new()),
        "amd" => ("", amd_module, String::new()),
        _ => bail!("unknown bundle kind {kind}; one of {KINDS:?}"),
    };
    let mut text = String::from(header);
    if kind == "systemjs" {
        text.push_str(&format!("System.register([\"m{}\"], function (_export, _context) {{\n  return {{ setters: [function () {{}}], execute: function () {{}} }};\n}});\n", modules - 1));
    }
    for index in 0..modules {
        text.push_str(&module(index, state));
    }
    text.push_str(&footer);
    Ok(text)
}

pub fn generate_kind(repo_root: &Path, kind: &str, modules: usize, seed: u64) -> Result<u64> {
    if modules == 0 {
        bail!("a bundle needs at least one module");
    }
    let mut state = seed | 1;
    let text = if kind == "webpack" {
        webpack_text(modules, &mut state)
    } else {
        listed_text(kind, modules, &mut state)?
    };
    let path = repo_root.join(output_for(kind));
    std::fs::create_dir_all(path.parent().context("generated dir")?)?;
    std::fs::write(&path, &text).with_context(|| format!("write {}", path.display()))?;
    Ok(text.len() as u64)
}

pub fn generate(repo_root: &Path, modules: usize, seed: u64) -> Result<u64> {
    generate_kind(repo_root, "webpack", modules, seed)
}
