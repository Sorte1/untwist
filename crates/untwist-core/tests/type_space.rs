use oxc_allocator::Allocator;
use untwist_core::parse::{error_count, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_core::type_space::{declares_an_interface, holds_type_space};

const PRODUCERS: &[&str] = &[
    "x as y;",
    "x satisfies y;",
    "label: x as y;",
    "export {}\nx satisfies y\n",
    "class A implements B {}",
    "class A extends B<T> {}",
    "class A extends (B)<T> {}",
    "var c = class extends B<T> {};",
];

const JAVASCRIPT_OR_FATAL: &[&str] = &[
    "export default interface I {}",
    "export default interface I extends J { m() }",
    "export {}\n{ export default interface I { m(b = J); } }\nvar J = g();",
    "{ export default interface I {} }",
    "function f() { export default interface I {} }",
    "var f = () => { export default interface I { m(a = b); [k](); } };",
    "class C { static { export default interface I {} } }",
    "if (a) { switch (b) { case 1: { label: { export default interface I {} } } } }",
    "function f() { return function () { { export default interface I extends J {} } }; }",
    "if (a) {} else export default interface I {}",
    "var f = () => { export default interface I { [k]: number } };",
    "class A extends B {}",
    "class C { [k] = 1 }",
    "class C { [k] }",
    "class C { [k: string]: any }",
    "class C { [k, j] }",
    "class C { accessor a = 1 }",
    "class C { private a = 1 }",
    "export default class C {}",
    "export default interface",
    "export default interface I<T> {}",
    "f<number>(1);",
    "let a = <T,>(x) => x;",
    "function f(this) {}",
    "function g(this: Window) {}",
    "let y: number = 1;",
    "function f(): number { return 1 }",
    "type Q = string;",
    "interface I {}",
    "declare const q;",
    "enum E { A }",
    "abstract class Z {}",
    "let n = x!;",
    "import type { T } from 'm';",
    "import x = require('m');",
    "export as namespace N;",
    "export = x;",
    "try {} catch (e: unknown) {}",
    "for (let x: T of y);",
    "a?.<T>();",
    "new C<T>();",
    "<div<T>></div>;",
    "async <T>(a) => a;",
    "(a: T) => a;",
    "(a): T => a;",
    "var o = { m(a: T) {} };",
    "class C<T> {}",
    "function f<T>() {}",
];

fn answers(source: &str) -> (bool, bool) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let full = holds_type_space(&parsed.program);
    let guard = if error_count(&parsed.diagnostics) > 0 {
        full
    } else {
        declares_an_interface(&build_scoping(&parsed.program).scoping)
    };
    (guard, full)
}

#[test]
fn every_type_node_javascript_parsing_admits_is_found() {
    for source in PRODUCERS {
        assert_eq!(answers(source), (true, true), "{source}");
    }
}

#[test]
fn the_errorless_answer_equals_the_full_walk_on_every_case() {
    for source in PRODUCERS.iter().chain(JAVASCRIPT_OR_FATAL) {
        let (cheap, full) = answers(source);
        assert_eq!(cheap, full, "{source}");
    }
    for source in JAVASCRIPT_OR_FATAL {
        assert_eq!(answers(source), (false, false), "{source}");
    }
}

fn fixture_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let skipped = ["generated", "corpus-npm", "real-world"]
            .iter()
            .any(|name| path.ends_with(name));
        if path.is_dir() && !skipped {
            fixture_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js")
            && std::fs::metadata(&path).unwrap().len() < 400_000
        {
            out.push(path);
        }
    }
}

#[test]
fn no_fixture_holds_a_type_node() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut paths = Vec::new();
    fixture_sources(&root, &mut paths);
    assert!(paths.len() > 200, "{}", paths.len());
    for path in paths {
        let source = std::fs::read_to_string(&path).unwrap();
        assert_eq!(answers(&source), (false, false), "{}", path.display());
    }
}
