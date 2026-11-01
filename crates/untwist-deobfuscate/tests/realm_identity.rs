mod common;

use oxc_allocator::Allocator;
use untwist_testing::node_rows::{self, node_or_skip, patched_after_rotator, Mode};

const HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c] + ([1, 2].join('') === '12' ? '!' : '?'); }
";

const JOIN_TEST: &str = "[1, 2].join('') === '12'";
const JOIN_PATCH: &str = "o.prototype.join = function () { return 'X'; };";
const NODE_PRINTS: &str = "log?\nHello World!?\ndeobfuscate?\nexit 0\n";

struct Identity {
    label: &'static str,
    decl: &'static str,
}

const IDENTITIES: [Identity; 8] = [
    Identity {
        label: "freeze of a made heir",
        decl: "var o = Object.freeze(Object.create(Array));",
    },
    Identity {
        label: "defineProperty of a made heir",
        decl: "var o = Object.defineProperty(Object.create(Array), 'x', { value: 1 });",
    },
    Identity {
        label: "freeze of the constructor",
        decl: "var o = Object.freeze(Array);",
    },
    Identity {
        label: "assign onto the constructor",
        decl: "var o = Object.assign(Array, {});",
    },
    Identity {
        label: "assign onto a made heir",
        decl: "var o = Object.assign(Object.create(Array), {});",
    },
    Identity {
        label: "seal of the constructor",
        decl: "var o = Object.seal(Array);",
    },
    Identity {
        label: "preventExtensions of the constructor",
        decl: "var o = Object.preventExtensions(Array);",
    },
    Identity {
        label: "defineProperties of the constructor",
        decl: "var o = Object.defineProperties(Array, {});",
    },
];

const SHADOWED: &str =
    "var Object = { freeze: function (f) { return {}; } }; var o = Object.freeze(Array);";

const ALIASES: [(&str, &str); 14] = [
    (
        "freeze as a value",
        "var fr = Object.freeze; var o = fr(Array);",
    ),
    (
        "seal as a value",
        "var fr = Object.seal; var o = fr(Array);",
    ),
    (
        "preventExtensions as a value",
        "var fr = Object.preventExtensions; var o = fr(Array);",
    ),
    (
        "assign as a value",
        "var fr = Object.assign; var o = fr(Array, {});",
    ),
    (
        "defineProperty as a value",
        "var fr = Object.defineProperty; var o = fr(Array, 'x', { value: 1 });",
    ),
    (
        "defineProperties as a value",
        "var fr = Object.defineProperties; var o = fr(Array, {});",
    ),
    (
        "destructured",
        "const { freeze } = Object; var o = freeze(Array);",
    ),
    (
        "computed key held",
        "var fr = Object['freeze']; var o = fr(Array);",
    ),
    ("call", "var o = Object.freeze.call(null, Array);"),
    ("apply", "var o = Object.freeze.apply(null, [Array]);"),
    (
        "bound",
        "var fr = Object.freeze.bind(null); var o = fr(Array);",
    ),
    ("sequence callee", "var o = (0, Object.freeze)(Array);"),
    (
        "alias of the global object",
        "var O = Object; var o = O.freeze(Array);",
    ),
    (
        "made heir through an alias",
        "var fr = Object.freeze; var o = fr(Object.create(Array));",
    ),
];

fn decode(patch: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(&format!(
        "{HEADER}{patch}\nconsole.log(__DECODE_0__(0x0));\n"
    ));
    common::run_ctors(&alloc, source, &[untwist_deobfuscate::decoded::boxed])
}

fn refused(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note.starts_with(untwist_deobfuscate::notes::BUILTIN_PATCHED))
}

fn deobfuscated(source: &str) -> String {
    common::run_report(source).0
}

#[test]
fn a_write_through_an_identity_built_ins_result_refuses_the_decode() {
    let mut wrong = Vec::new();
    for Identity { label, decl } in IDENTITIES {
        let (code, notes) = decode(&format!("{decl} {JOIN_PATCH}"));
        if !refused(&notes) || !code.contains("__DECODE_0__(0)") {
            wrong.push(format!("{label}: {notes:?}\n{code}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_aliased_identity_built_in_keeps_the_decode_unfolded() {
    let mut wrong = Vec::new();
    for (label, decl) in ALIASES {
        let (code, notes) = decode(&format!("{decl} {JOIN_PATCH}"));
        if !code.contains("__DECODE_0__(0)") {
            wrong.push(format!("{label}: {notes:?}\n{code}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_aliased_identity_built_in_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let transform = |source: &str| deobfuscated(source);
    let modes: [Mode<'_>; 1] = [("deobfuscate", &transform)];
    let mut programs: Vec<(String, String)> = ALIASES
        .iter()
        .map(|(label, decl)| {
            (
                label.to_string(),
                patched_after_rotator(JOIN_TEST, &format!("{decl} {JOIN_PATCH}")),
            )
        })
        .collect();
    programs.push((
        "shadowed Object".to_string(),
        patched_after_rotator(JOIN_TEST, &format!("{SHADOWED} {JOIN_PATCH}")),
    ));
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in programs.iter().take(ALIASES.len()) {
        let seen = node_rows::printed(&node, dir.path(), source).unwrap();
        assert!(
            seen.starts_with(NODE_PRINTS),
            "{label}: node printed {seen:?}"
        );
    }
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_identity_built_ins_result_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let transform = |source: &str| deobfuscated(source);
    let modes: [Mode<'_>; 1] = [("deobfuscate", &transform)];
    let programs: Vec<(String, String)> = IDENTITIES
        .iter()
        .map(|i| {
            (
                i.label.to_string(),
                patched_after_rotator(JOIN_TEST, &format!("{} {JOIN_PATCH}", i.decl)),
            )
        })
        .collect();
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in &programs {
        let seen = node_rows::printed(&node, dir.path(), source).unwrap();
        assert!(
            seen.starts_with(NODE_PRINTS),
            "{label}: node printed {seen:?}"
        );
    }
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_frozen_primitive_is_itself_and_a_non_writable_assign_throws_as_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let transform = |source: &str| deobfuscated(source);
    let modes: [Mode<'_>; 1] = [("deobfuscate", &transform)];
    let frozen = patched_after_rotator(JOIN_TEST, "var o = Object.freeze(1); console.log(o);");
    let throwing = patched_after_rotator(
        JOIN_TEST,
        "var o = Object.assign(Array, { prototype: Object.prototype }); o.prototype.join = function () { return 'X'; };",
    );
    let programs = vec![
        ("freeze of a primitive".to_string(), frozen.clone()),
        (
            "assign over a non-writable prototype".to_string(),
            throwing.clone(),
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let seen = node_rows::printed(&node, dir.path(), &frozen).unwrap();
    assert!(
        seen.starts_with("1\nlog!\nHello World!!\ndeobfuscate!\nexit 0\n"),
        "{seen:?}"
    );
    let seen = node_rows::printed(&node, dir.path(), &throwing).unwrap();
    assert!(
        seen.contains("TypeError") || seen.contains("exit 1"),
        "{seen:?}"
    );
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
