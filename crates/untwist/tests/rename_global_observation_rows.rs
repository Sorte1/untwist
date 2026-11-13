use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const WRAPPER: &str = "var a, b;\na || (a = 1, b = { v: 1 });\n";
const DECLS: &str = "var q = 1;\nfunction zz() { return 2; }\n";

const OBSERVED: &[(&str, &str)] = &[
    ("in-globalThis", "console.log(\"q\" in globalThis, \"zz\" in globalThis);"),
    ("in-this", "console.log(\"q\" in this, \"zz\" in this);"),
    ("in-unknown-key", "var k = String.fromCharCode(113); console.log(k in globalThis);"),
    ("hasOwnProperty", "console.log(globalThis.hasOwnProperty(\"zz\"), this.hasOwnProperty(\"q\"));"),
    ("hasOwnProperty-call", "console.log(Object.prototype.hasOwnProperty.call(globalThis, \"q\"));"),
    ("Object.hasOwn", "console.log(Object.hasOwn(globalThis, \"q\"));"),
    ("Object.keys", "console.log(Object.keys(globalThis).indexOf(\"q\") >= 0);"),
    ("Object.keys-this", "console.log(Object.keys(this).filter(function (k) { return k.length < 3; }).join());"),
    ("Object.keys-either", "console.log(Object.keys(typeof window !== \"undefined\" ? window : globalThis).indexOf(\"zz\"));"),
    ("getOwnPropertyNames", "console.log(Object.getOwnPropertyNames(globalThis).includes(\"q\"));"),
    ("Reflect.ownKeys", "console.log(Reflect.ownKeys(globalThis).includes(\"zz\"));"),
    ("Reflect.has", "console.log(Reflect.has(globalThis, \"q\"));"),
    ("for-in", "var seen = []; for (var k in globalThis) if (k.length < 3) seen.push(k); console.log(seen.join());"),
    ("Object.entries", "console.log(Object.entries(globalThis).filter(function (e) { return e[0].length < 3; }).length);"),
    ("getOwnPropertyDescriptor", "console.log(JSON.stringify(Object.getOwnPropertyDescriptor(globalThis, \"q\")));"),
    ("getOwnPropertyDescriptors", "console.log(Object.keys(Object.getOwnPropertyDescriptors(globalThis)).includes(\"q\"));"),
];

const UNSAFE_READS: &[(&str, &str)] = &[
    ("static-read", "console.log(globalThis.q, globalThis.zz());"),
    ("keyed-read", "var k = \"q\"; console.log(globalThis[k]);"),
    (
        "this-call",
        "try { console.log(this.zz()); } catch (e) { console.log(\"threw\"); }",
    ),
    (
        "indirect-eval",
        "try { console.log((0, eval)(\"q + zz()\")); } catch (e) { console.log(\"threw\"); }",
    ),
    (
        "function-ctor",
        "try { console.log(new Function(\"return q\")()); } catch (e) { console.log(\"threw\"); }",
    ),
];

const PENDING: &[(&str, &str)] = &[
    (
        "base-equals-name",
        "(function () { var a, e; function requireE() { if (a) return e; a = 1; e = { v: 1 }; return e; } (function () { var b, x; function requireE() { if (b) return x; b = 1; x = { v: 2 }; return x; } requireE(); console.log(e); })(); requireE(); })();",
    ),
    (
        "base-equals-name-sibling",
        "(function () { var a, e; function requireE() { if (a) return e; a = 1; e = { v: 1 }; return e; } var c, y; function requireF() { if (c) return y; c = 1; y = { v: 3 }; return y; } (function () { var b, x; function requireE() { if (b) return x; b = 1; x = { v: 2 }; return x; } requireE(); console.log(e, y); })(); requireE(); requireF(); })();",
    ),
];

fn out_of_names() -> String {
    let taken: Vec<String> = std::iter::once("typeof hasRequiredE".to_string())
        .chain((2..64).map(|n| format!("typeof hasRequiredE{n}")))
        .collect();
    format!(
        "(function () {{ var e, x, n = 0; function requireE() {{ if (e) return x; e = 1; n++; x = 0; return x; }} requireE(); requireE(); console.log(n); console.log({}); }})();",
        taken.join(", ")
    )
}

fn run(
    source: &str,
    rename: RenameMode,
    unsafe_transforms: bool,
    disabled: Vec<&'static str>,
) -> String {
    Pipeline::new(Options {
        rename,
        unsafe_transforms,
        disabled_passes: disabled,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

const RENAMES: [(&str, RenameMode); 5] = [
    ("no rename", RenameMode::None),
    ("mangle", RenameMode::Mangle),
    ("simple", RenameMode::Simple),
    ("unique", RenameMode::Unique),
    ("smart", RenameMode::Smart),
];

fn every_lane_and_mode(node: &std::path::Path, programs: &[(String, String)]) -> Vec<String> {
    let mut wrong = Vec::new();
    for unsafe_transforms in [false, true] {
        for (label, rename) in RENAMES {
            let transform = move |source: &str| run(source, rename, unsafe_transforms, Vec::new());
            let modes: [Mode<'_>; 1] = [(label, &transform)];
            wrong.extend(
                mismatches(node, programs, &modes)
                    .into_iter()
                    .map(|row| format!("unsafe={unsafe_transforms} {row}")),
            );
        }
    }
    wrong
}

fn rows(prefix: &str, tests: &[(&str, &str)]) -> Vec<(String, String)> {
    tests
        .iter()
        .flat_map(|(label, test)| {
            [
                (format!("decls {label}"), format!("{DECLS}{test}")),
                (
                    format!("wrapper {label}"),
                    format!(
                        "{WRAPPER}{}",
                        test.replace("\"q\"", "\"a\"")
                            .replace("\"zz\"", "\"b\"")
                            .replace("113", "97")
                    ),
                ),
            ]
        })
        .map(|(label, program)| (format!("{prefix} {label}"), program))
        .collect()
}

#[test]
fn a_name_observed_on_the_global_object_keeps_every_script_global() {
    for (label, test) in OBSERVED {
        for unsafe_transforms in [false, true] {
            for (mode, rename) in RENAMES {
                let code = run(
                    &format!("{DECLS}{test}"),
                    rename,
                    unsafe_transforms,
                    Vec::new(),
                );
                assert!(
                    code.contains("var q = 1"),
                    "{label} {mode} unsafe={unsafe_transforms}:\n{code}"
                );
                assert!(
                    code.contains("function zz("),
                    "{label} {mode} unsafe={unsafe_transforms}:\n{code}"
                );
            }
        }
    }
}

#[test]
fn the_unsafe_lane_keeps_script_globals_the_program_reads_off_the_global_object() {
    for (label, test) in UNSAFE_READS {
        for (mode, rename) in RENAMES {
            let code = run(&format!("{DECLS}{test}"), rename, true, Vec::new());
            assert!(code.contains("var q = 1"), "{label} {mode}:\n{code}");
        }
    }
}

#[test]
fn global_observation_rows_print_the_same_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows("observed", OBSERVED);
    let wrong = every_lane_and_mode(&node, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn unsafe_global_read_rows_print_the_same_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows("read", UNSAFE_READS);
    let wrong = every_lane_and_mode(&node, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_observation_guard_is_load_bearing() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![(
        "in-globalThis".to_string(),
        format!("{DECLS}{}", OBSERVED[0].1),
    )];
    let unguarded = |source: &str| {
        run(
            source,
            RenameMode::Simple,
            false,
            vec!["rename-keeps-globals"],
        )
    };
    let modes: [Mode<'_>; 1] = [("unguarded", &unguarded)];
    assert_eq!(mismatches(&node, &programs, &modes).len(), 1);
}

#[test]
fn wrapper_bindings_never_share_a_name_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs: Vec<(String, String)> = PENDING
        .iter()
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    programs.push(("out-of-names".to_string(), out_of_names()));
    let wrong = every_lane_and_mode(&node, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
