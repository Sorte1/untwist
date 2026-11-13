use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{walk_js, VisitJs};
use untwist_core::parse::parse_program;
use untwist_unpack::browserify::detect::is_browserify;
use untwist_unpack::browserify::forms::{HEAD_FORMS, PRELUDE_FORMS};
use untwist_unpack::browserify::notes::{HOST_OBJECT, NOT_PROVEN};
use untwist_unpack::browserify::prelude::{frame, head_text, prelude_text, Frame, NAME};
use untwist_unpack::webpack::machinery::form_digest;
use untwist_unpack::{unpack_inputs, BundleKind, UnpackOptions};

const PRELUDE: &str = "(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c=\"function\"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error(\"Cannot find module '\"+i+\"'\");throw a.code=\"MODULE_NOT_FOUND\",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u=\"function\"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()";

const MODULES: &str = "{1:[function(require,module,exports){console.log(require(\"./b\"))},{\"./b\":2}],2:[function(require,module,exports){BODY},{}]}";

const UMD_HEAD: &str = "(function(f){if(typeof exports===\"object\"&&typeof module!==\"undefined\"){module.exports=f()}else if(typeof define===\"function\"&&define.amd){define([],f)}else{var g;if(typeof window!==\"undefined\"){g=window}else if(typeof global!==\"undefined\"){g=global}else if(typeof self!==\"undefined\"){g=self}else{g=this}g.NAME = f()}})";

const REFERENCE: [&str; 3] = [
    "browserify.js",
    "browserify-2.js",
    "browserify-webpack-nested.js",
];

struct Outcome {
    kind: Option<BundleKind>,
    notes: Vec<String>,
}

impl Outcome {
    fn refused_with(&self, text: &str) -> bool {
        self.kind.is_none() && self.notes.iter().any(|note| note.contains(text))
    }
}

fn outcome(source: &str) -> Outcome {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut programs = vec![parsed.program];
    let (bundle, notes) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    Outcome {
        kind: bundle.map(|bundle| bundle.kind),
        notes: notes.into_iter().map(|note| note.message).collect(),
    }
}

fn bundle(body: &str) -> String {
    format!("{PRELUDE}({}, {{}}, [1]);", MODULES.replace("BODY", body))
}

fn standalone(head: &str, entry: &str) -> String {
    let modules = MODULES.replace("BODY", "module.exports = 2;");
    format!("{head}(function(){{var define,module,exports;return {PRELUDE}({modules},{{}},[1])({entry})}});")
}

#[test]
fn browser_packs_own_prelude_is_taken() {
    let found = outcome(&bundle("module.exports = 2;"));
    assert_eq!(
        found.kind,
        Some(BundleKind::Browserify),
        "{:?}",
        found.notes
    );
}

#[test]
fn a_prelude_that_is_not_browser_packs_refuses_the_bundle() {
    for prelude in [
        "(function (e, n, t) { console.log(\"fake prelude\", Object.keys(e).join()); })",
        &PRELUDE.replace("for(var u", "console.log(1);for(var u"),
        &PRELUDE.replace("o(t[i])", "o(t[t.length-1-i])"),
        &PRELUDE.replace("call(p.exports", "call(null"),
    ] {
        let source = format!("{prelude}({}, {{}}, [1]);", MODULES.replace("BODY", ""));
        let found = outcome(&source);
        assert!(
            found.refused_with(NOT_PROVEN),
            "{prelude}: {:?}",
            found.notes
        );
    }
}

#[test]
fn a_bundle_inside_other_code_is_refused() {
    let plain = bundle("module.exports = 2;");
    for source in [
        format!("console.log(\"before\");\n{plain}"),
        format!("{plain}\nconsole.log(\"after\");"),
        format!("var x = {}", plain),
        format!("if (globalThis.flag) {plain}"),
        format!("require = {plain}"),
        format!("\"use strict\";\n{plain}"),
        format!("(function () {{ {plain} }})();"),
    ] {
        let found = outcome(&source);
        assert!(
            found.refused_with(NOT_PROVEN),
            "{source}: {:?}",
            found.notes
        );
    }
}

#[test]
fn a_bundle_whose_cache_or_entries_differ_from_browser_packs_is_not_taken() {
    let modules = MODULES.replace("BODY", "");
    for arguments in [
        format!("{modules}, {{1: {{exports: 3}}}}, [1]"),
        format!("{modules}, {{}}, [1, globalThis.next]"),
        format!("{modules}, {{}}, [...[1]]"),
    ] {
        let found = outcome(&format!("{PRELUDE}({arguments});"));
        assert_eq!(found.kind, None, "{arguments}: {:?}", found.notes);
    }
}

#[test]
fn browserifys_standalone_wrapper_is_taken() {
    let head = UMD_HEAD.replace("NAME", "matrix");
    let found = outcome(&standalone(&head, "1"));
    assert_eq!(
        found.kind,
        Some(BundleKind::Browserify),
        "{:?}",
        found.notes
    );
    let renamed = UMD_HEAD.replace("NAME", "$other_Lib2");
    let found = outcome(&standalone(&renamed, "1"));
    assert_eq!(
        found.kind,
        Some(BundleKind::Browserify),
        "{:?}",
        found.notes
    );
}

#[test]
fn a_standalone_wrapper_that_is_not_browserifys_is_refused() {
    let head = UMD_HEAD.replace("NAME", "matrix");
    for source in [
        standalone(&head, "2"),
        standalone(
            &head.replace("module.exports=f()", "module.exports=[f()]"),
            "1",
        ),
        standalone(&head.replace("g.matrix = f()", "g.a.b = f()"), "1"),
        standalone(&head.replace("g.matrix = f()", "g[\"matrix\"] = f()"), "1"),
        standalone(
            "(function(f){console.log(\"wrapper\");module.exports=f()})",
            "1",
        ),
        standalone(&head, "1").replace("var define,module,exports;", "console.log(1);"),
    ] {
        let found = outcome(&source);
        assert!(
            found.refused_with(NOT_PROVEN),
            "{source}: {:?}",
            found.notes
        );
    }
}

#[test]
fn a_module_that_reads_nodes_module_object_refuses_the_bundle() {
    for (body, what) in [
        ("if (!module.parent) console.log(1);", "reads module.parent"),
        ("console.log(module.id, module.loaded);", "reads module.id"),
        ("console.log(module.filename);", "reads module.filename"),
        ("console.log(module.children);", "reads module.children"),
        ("console.log(module.paths);", "reads module.paths"),
        ("console.log(module.require);", "reads module.require"),
        ("console.log(module?.exports);", "reads module.exports"),
        (
            "console.log(module[\"id\"]);",
            "reads a computed property of module",
        ),
        (
            "console.log(Object.keys(module));",
            "uses module as a value",
        ),
        ("var m = module; m.exports = 1;", "uses module as a value"),
        ("console.log(require.cache);", "reads require.cache"),
        (
            "console.log(require.main === undefined);",
            "reads require.main",
        ),
        (
            "var r = require; r(\"./b\");",
            "uses require other than by calling it",
        ),
    ] {
        let found = outcome(&bundle(body));
        let note = format!("module 2: it {what}, {HOST_OBJECT}");
        assert!(
            found.kind.is_none() && found.notes.contains(&note),
            "{body}: {:?}",
            found.notes
        );
    }
}

#[test]
fn module_exports_and_typeof_checks_are_what_both_give() {
    for body in [
        "module.exports = 2;",
        "module.exports.a = 2; exports.b = 3;",
        "module[\"exports\"] = 2;",
        "if (typeof module === \"object\" && typeof require === \"function\") module.exports = 2;",
        "(function (module) { console.log(module.parent); })({});",
        "this.a = module.exports === exports;",
    ] {
        let found = outcome(&bundle(body));
        assert_eq!(
            found.kind,
            Some(BundleKind::Browserify),
            "{body}: {:?}",
            found.notes
        );
    }
}

#[test]
fn the_reference_samples_with_browser_packs_prelude_are_taken() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference/unpack");
    for name in REFERENCE {
        let source = std::fs::read_to_string(root.join(name)).expect("sample");
        let found = outcome(&source);
        assert_eq!(
            found.kind,
            Some(BundleKind::Browserify),
            "{name}: {:?}",
            found.notes
        );
    }
    let cocos = std::fs::read_to_string(root.join("browserify-cocos2d.js")).expect("sample");
    assert!(outcome(&cocos).refused_with(NOT_PROVEN));
}

fn recorded_texts() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/browserify_forms.txt");
    std::fs::read_to_string(path)
        .expect("forms")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (role, text) = line.split_once('\t').expect("role");
            let mut unescaped = String::new();
            let mut chars = text.chars();
            while let Some(c) = chars.next() {
                match (c, c == '\\') {
                    (_, true) => match chars.next() {
                        Some('n') => unescaped.push('\n'),
                        Some(other) => unescaped.push(other),
                        None => {}
                    },
                    (c, false) => unescaped.push(c),
                }
            }
            (role.to_string(), unescaped)
        })
        .collect()
}

#[test]
fn every_recorded_digest_has_its_form_text() {
    let texts = recorded_texts();
    let preludes: Vec<String> = texts
        .iter()
        .filter(|(role, _)| role == "PRELUDE")
        .map(|(_, text)| form_digest(text))
        .collect();
    assert_eq!(preludes, PRELUDE_FORMS);
    let heads: Vec<(usize, String, String)> = texts
        .iter()
        .filter(|(role, _)| role == "HEAD")
        .map(|(_, text)| {
            let (prefix, suffix) = text.split_once(NAME).expect("a name slot");
            (prefix.len(), form_digest(prefix), form_digest(suffix))
        })
        .collect();
    let recorded: Vec<(usize, String, String)> = HEAD_FORMS
        .iter()
        .map(|(len, prefix, suffix)| (*len, prefix.to_string(), suffix.to_string()))
        .collect();
    assert_eq!(heads, recorded);
}

struct Dump<'k, 'a> {
    program: &'k Program<'a>,
    lines: Vec<String>,
}

fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

impl<'a> Dump<'_, 'a> {
    fn record(&mut self, expr: &Expression<'a>) {
        let Expression::CallExpression(call) = expr else {
            return;
        };
        let source_text = self.program.source_text;
        if let Some(text) = prelude_text(&call.callee, source_text) {
            self.lines.push(format!("PRELUDE\t{}", escaped(&text)));
        }
        if let Ok(Frame::Standalone { head, .. }) = frame(self.program, expr) {
            if let Some(text) = head_text(head, source_text) {
                self.lines.push(format!("HEAD\t{}", escaped(&text)));
            }
        }
    }
}

impl<'a> VisitJs<'a> for Dump<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if is_browserify(it) {
            self.record(it);
        }
        walk_js::walk_expression(self, it);
    }
}

#[test]
#[ignore]
fn dump_browserify_forms() {
    let root = std::env::var("UNTWIST_FORMS_ROOT").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/browserify-preludes"),
        |root| Path::new(&root).to_path_buf(),
    );
    let mut files = Vec::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("dir") {
            let path = entry.expect("entry").path();
            match path.is_dir() {
                true => pending.push(path),
                false if path.extension().is_some_and(|ext| ext == "js") => files.push(path),
                false => {}
            }
        }
    }
    files.sort();
    let mut lines = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        let mut dump = Dump {
            program: &parsed.program,
            lines: Vec::new(),
        };
        dump.visit_program(&parsed.program);
        lines.extend(dump.lines);
    }
    for line in lines {
        println!("{line}");
    }
}
