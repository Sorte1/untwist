use std::path::{Path, PathBuf};

use untwist::{parse_modern_baseline, Options, Pipeline};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const USAGE: &str = "babel-usage.core-js-3.39.0.rollup-cjs.js";
const ENTRY: &str = "babel-entry.core-js-3.39.0.rollup-cjs.js";
const BASELINES: [&str; 5] = [
    "es2017",
    "es2022",
    "node 22",
    "chrome 120, firefox 121, safari 17",
    "deno 1.40",
];

struct Probe {
    name: &'static str,
    base: &'static str,
    edits: &'static [(&'static str, &'static str)],
    tail: &'static str,
}

const PROBES: &[Probe] = &[
    Probe {
        name: "aggregator-runs-a-needed-module",
        base: USAGE,
        edits: &[("\n\trequireEs_json_stringify();\n", "\n")],
        tail: "\tconsole.log(\"json\", JSON.stringify(\"\\ud800\"), typeof Symbol());\n",
    },
    Probe {
        name: "removed-module-runs-a-needed-module",
        base: ENTRY,
        edits: &[
            ("\n\trequireEs_promise_try();\n", "\n"),
            (
                "hasRequiredEs_regexp_test = 1;\n\t\t// TODO: Remove from `core-js@4` since it's moved to entry points\n\t\trequireEs_regexp_exec();",
                "hasRequiredEs_regexp_test = 1;\n\t\t// TODO: Remove from `core-js@4` since it's moved to entry points\n\t\trequireEs_promise_try();",
            ),
        ],
        tail: "\tconsole.log(\"Promise.try:\", typeof Promise.try);\n",
    },
    Probe {
        name: "user-state-functions",
        base: USAGE,
        edits: &[],
        tail: "\tvar cache;\n\tfunction init() { cache = { n: 1 }; return cache; }\n\tfunction get() { return cache; }\n\tvar started = init();\n\tconsole.log(\"user state:\", get().n);\n",
    },
    Probe {
        name: "user-wrapper-shaped-functions",
        base: USAGE,
        edits: &[],
        tail: "\tvar seen, hasSeen;\n\tfunction mark() { if (hasSeen) return seen; hasSeen = 1; seen = true; return seen; }\n\tvar marked = mark();\n\tconsole.log(\"marked:\", hasSeen, seen);\n",
    },
    Probe {
        name: "user-terser-wrapper-shaped-functions",
        base: USAGE,
        edits: &[],
        tail: "\tvar box, hasBox, flag, hasFlag;\n\tfunction boxed() { if (hasBox) { return box; } else { hasBox = 1; return box = {}; } }\n\tfunction flagged() { if (!hasFlag) { hasFlag = 1; flag = true; } return flag; }\n\tvar madeBox = boxed(), madeFlag = flagged();\n\tconsole.log(\"wrappers:\", hasBox, typeof box, hasFlag, flag);\n",
    },
    Probe {
        name: "needed-module-run-through-an-unused-var",
        base: ENTRY,
        edits: &[(
            "\n\trequireEs_promise_try();\n",
            "\n\tvar es_promise_try$1 = requireEs_promise_try();\n",
        )],
        tail: "\tconsole.log(\"Promise.try:\", typeof Promise.try);\n",
    },
    Probe {
        name: "helper-calls-in-odd-positions",
        base: USAGE,
        edits: &[],
        tail: concat!(
            r#"
	(function () {
		var uncurry$ = requireFunctionUncurryThis();
		var sl$ = uncurry$("".slice);
		var gbi$ = requireGetBuiltIn();
		var wks$ = requireWellKnownSymbol();
		var isC$ = requireIsCallable();
		var ho$ = requireHasOwnProperty();
		function T(name, f) { try { console.log(name, String(f())); } catch (e) { console.log(name, "threw", e.name); } }
"#,
            r#"		T("shadow", function () { return (function (String) { return sl$("abcdef", 2); })({ prototype: { slice: { call: function () { return "WRONG"; } } } }); });
		T("shadow Array", function () { var push$ = uncurry$([].push); return (function (Array) { var a = []; return push$(a, 1); })({ prototype: { push: { call: function () { return "WRONG"; } } } }); });
		T("plain", function () { return sl$("abcdef", 2) + " " + ho$({ x: 1 }, "x") + " " + typeof gbi$("Reflect", "ownKeys") + " " + String(wks$("iterator")); });
		T("undeclared", function () { return isC$(notDeclaredAnywhere); });
		T("delete member", function () { var o = { r: delete gbi$("Object", "groupBy") }; return o.r + " " + typeof Object.groupBy; });
		T("delete symbol", function () { return delete wks$("iterator"); });
		T("delete parens", function () { return (delete (gbi$("Object", "keys"))) + " " + typeof Object.keys; });
		T("typeof", function () { return typeof gbi$("Symbol") + " " + void gbi$("Object", "keys"); });
		T("early", function () { return early(); });
		function early() { return typeof late$("iterator"); }
		var late$ = requireWellKnownSymbol();
		T("late", function () { return String(late$("iterator")); });
	})();
	(function () {
		"use strict";
		var gbi$ = requireGetBuiltIn();
		try { console.log("strict delete", delete gbi$("AggregateError"), typeof AggregateError); } catch (e) { console.log("strict delete threw", e.name); }
	})();
	(function () {
		var early$ = function () { try { return String(lateA$("iterator")); } catch (e) { return e.name; } };
		console.log("closure before", early$());
		var lateA$ = requireWellKnownSymbol();
		var after$ = requireWellKnownSymbol();
		console.log("closure after", early$(), String(after$("iterator")));
	})();
	(function () {
		var first = requireWellKnownSymbol();
		console.log("redeclared", String(first("iterator")));
		var first = function () { return "replaced"; };
		console.log("redeclared", first("iterator"));
	})();
"#
        ),
    },
    Probe {
        name: "core-js-pure-path",
        base: USAGE,
        edits: &[],
        tail: r#"
	var path$9;
	var hasRequiredPath$9;

	function requirePath$9 () {
		if (hasRequiredPath$9) return path$9;
		hasRequiredPath$9 = 1;
		path$9 = {};
		return path$9;
	}

	var getBuiltIn$9;
	var hasRequiredGetBuiltIn$9;

	function requireGetBuiltIn$9 () {
		if (hasRequiredGetBuiltIn$9) return getBuiltIn$9;
		hasRequiredGetBuiltIn$9 = 1;
		var path = requirePath$9();
		var global = requireGlobalThis();

		var aFunction = function (variable) {
		  return typeof variable == 'function' ? variable : undefined;
		};

		getBuiltIn$9 = function (namespace, method) {
		  return arguments.length < 2 ? aFunction(path[namespace]) || aFunction(global[namespace])
		    : path[namespace] && path[namespace][method] || global[namespace] && global[namespace][method];
		};
		return getBuiltIn$9;
	}

	function pureExport$9(name, value) {
		var path = requirePath$9();
		(path.Promise || (path.Promise = function PurePromise() {}))[name] = value;
	}
	pureExport$9("try", function tryPolyfill() { return "pure Promise.try"; });
	(function () {
		var gbi$9 = requireGetBuiltIn$9();
		console.log("pure:", gbi$9("Promise").name, typeof gbi$9("Promise", "try"));
	})();
"#,
    },
];

fn fixture(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/modern")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

fn source(probe: &Probe) -> String {
    let mut text = fixture(probe.base);
    for (from, to) in probe.edits {
        assert_eq!(text.matches(from).count(), 1, "{}: {from:?}", probe.name);
        text = text.replacen(from, to, 1);
    }
    let end = text.trim_end().rfind("})();").unwrap();
    text.insert_str(end, probe.tail);
    text
}

struct Ran {
    code: String,
    notes: Vec<String>,
}

fn untwist(source: &str, baseline: &str) -> Ran {
    let options = Options {
        unpack: false,
        unsafe_transforms: true,
        modern: Some(parse_modern_baseline(baseline).unwrap()),
        ..Options::default()
    };
    let output = Pipeline::new(options).run(source);
    Ran {
        code: output.code().to_string(),
        notes: output
            .notes
            .iter()
            .filter(|note| note.pass == "modern")
            .map(|note| note.message.clone())
            .collect(),
    }
}

fn printed(node: &Path, dir: &Path, code: &str) -> String {
    let path = dir.join("program.js");
    std::fs::write(&path, code).unwrap();
    let observed = run(node, &path).unwrap().expect("node finished");
    format!("{}exit {:?}\n", observed.stdout, observed.exit)
}

fn probe(name: &str) -> &'static Probe {
    PROBES.iter().find(|probe| probe.name == name).unwrap()
}

#[test]
fn every_probe_prints_what_its_input_prints_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for probe in PROBES {
        let input = source(probe);
        let before = printed(&node, dir.path(), &input);
        for baseline in BASELINES {
            let ran = untwist(&input, baseline);
            let after = printed(&node, dir.path(), &ran.code);
            if after != before {
                wrong.push(format!(
                    "{} --modern {baseline}:\n  node printed {before:?}\n  output printed {after:?}",
                    probe.name
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn note<'r>(ran: &'r Ran, prefix: &str, module: &str) -> Option<&'r String> {
    ran.notes
        .iter()
        .find(|note| note.starts_with(prefix) && note.contains(&format!("modules/{module}.js")))
}

#[test]
fn a_module_that_runs_a_needed_module_is_kept() {
    let ran = untwist(&source(probe("aggregator-runs-a-needed-module")), "es2017");
    let stringify = note(&ran, "kept", "es.json.stringify");
    assert!(stringify.is_some(), "{}", ran.notes.join("\n"));
    let symbol = note(&ran, "kept", "es.symbol").unwrap();
    assert!(
        symbol.ends_with("it runs a core-js module that is kept"),
        "{symbol}"
    );
    assert!(ran.code.contains("requireEs_json_stringify();"));
}

#[test]
fn a_needed_module_run_only_by_a_removed_module_still_runs() {
    let ran = untwist(
        &source(probe("removed-module-runs-a-needed-module")),
        "node 22",
    );
    assert!(
        note(&ran, "kept", "es.promise.try").is_some(),
        "{}",
        ran.notes.join("\n")
    );
    assert!(note(&ran, "removed", "es.promise.try").is_none());
}

fn plain(source: &str) -> String {
    let options = Options {
        unpack: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn same_count(name: &str, ran: &Ran, texts: &[&str]) {
    let without = plain(&source(probe(name)));
    for text in texts {
        let expected = without.matches(text).count();
        assert!(
            expected > 0,
            "{name}: {text} is not in the output without --modern"
        );
        assert_eq!(ran.code.matches(text).count(), expected, "{name}: {text}");
    }
}

#[test]
fn user_functions_and_their_results_are_never_removed() {
    let name = "user-state-functions";
    let ran = untwist(&source(probe(name)), "node 22");
    same_count(
        name,
        &ran,
        &["function init()", "= init()", "function get()"],
    );
    let name = "user-wrapper-shaped-functions";
    let ran = untwist(&source(probe(name)), "node 22");
    same_count(name, &ran, &["function mark()", "= mark()", "hasSeen"]);
    let name = "user-terser-wrapper-shaped-functions";
    let ran = untwist(&source(probe(name)), "node 22");
    same_count(
        name,
        &ran,
        &[
            "function boxed()",
            "function flagged()",
            "boxed()",
            "flagged()",
        ],
    );
}

#[test]
fn a_kept_module_run_through_an_unused_var_keeps_its_run() {
    let ran = untwist(
        &source(probe("needed-module-run-through-an-unused-var")),
        "node 22",
    );
    assert!(ran.code.contains("requireEs_promise_try()"));
    assert!(note(&ran, "removed", "es.promise.try").is_none());
}

#[test]
fn helper_calls_under_delete_shadowed_or_before_their_alias_stay_calls() {
    let name = "helper-calls-in-odd-positions";
    let ran = untwist(&source(probe(name)), "node 22");
    same_count(
        name,
        &ran,
        &[
            "push$(a, 1)",
            "isC$(notDeclaredAnywhere)",
            "delete gbi$(\"Object\", \"groupBy\")",
            "delete wks$(\"iterator\")",
            "delete gbi$(\"Object\", \"keys\")",
            "delete gbi$(\"AggregateError\")",
            "typeof late$(\"iterator\")",
            "String(late$(\"iterator\"))",
            "first(\"iterator\")",
            "lateA$(\"iterator\")",
        ],
    );
    assert_eq!(ran.code.matches("sl$(\"abcdef\", 2)").count(), 1);
    assert!(ran.code.contains("String(Symbol.iterator)"));
    assert!(ran
        .code
        .contains("String.prototype.slice.call(\"abcdef\", 2)"));
}

#[test]
fn core_js_pure_is_left_alone() {
    let ran = untwist(&source(probe("core-js-pure-path")), "node 22");
    assert!(
        ran.notes.iter().any(|note| note.contains("core-js-pure")),
        "{}",
        ran.notes.join("\n")
    );
    assert!(ran.code.contains("gbi$9(\"Promise\").name"));
    assert!(!ran.notes.iter().any(|note| note.starts_with("removed")));
}
