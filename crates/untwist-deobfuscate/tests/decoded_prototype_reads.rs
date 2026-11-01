mod common;

use oxc_allocator::Allocator;

const PROTO_HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
";

const INDEXING: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c]; }\n";
const JOINING: &str =
    "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return [e[c]].join(''); }\n";
const CONCATENATING: &str =
    "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return '' + [e[c]]; }\n";
const HOLEY: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); var p = []; p[1] = e[c]; return p[0] === undefined ? p[1] : p[0]; }\n";
const STRING_INDEX: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); var i = e[c]; var x = i[20]; return x === undefined ? i : x; }\n";
const STRING_MEMBER: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); var i = e[c]; return i.foo === undefined ? i : i.foo; }\n";
const STRING_METHOD: &str =
    "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c].toUpperCase(); }\n";
const ENUMERATING: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); var n = 0; for (var k in {}) { n++; } return n ? 'enum' : e[c]; }\n";
const CACHING: &str = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); __DECODE_0__['k' + c] = e[c]; return e[c]; }\n";
const NUMBERING: &str =
    "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c] + (1.5).toFixed(0); }\n";

fn proto_notes(decoder: &str, call: &str, patch: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = format!("{PROTO_HEADER}{decoder}{patch}\nconsole.log(__DECODE_0__({call}));\n");
    common::run_ctors(&alloc, &source, &[untwist_deobfuscate::decoded::boxed])
}

fn refused(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note.starts_with(untwist_deobfuscate::notes::BUILTIN_PATCHED))
}

const REFUSING_WRITES: [(&str, &str, &str); 22] = [
    (INDEXING, "0x7", "Array.prototype[7] = 'patched';"),
    (INDEXING, "0x7", "Array['prototype']['7'] = 'patched';"),
    (INDEXING, "0x7", "Object.prototype[7] = 'patched';"),
    (
        INDEXING,
        "0x7",
        "Object.defineProperty(Array.prototype, '7', { get: function () { return 'g'; } });",
    ),
    (INDEXING, "0x7", "[].__proto__[7] = 'patched';"),
    (INDEXING, "0x7", "Object.setPrototypeOf(Array.prototype, null);"),
    (INDEXING, "0x7", "function g(p) { p[7] = 'x'; } g(Array.prototype);"),
    (
        JOINING,
        "0x0",
        "var P = Array.prototype; P.join = function () { return 'J'; };",
    ),
    (
        JOINING,
        "0x0",
        "var P = Array['prototype']; P['join'] = function () { return 'J'; };",
    ),
    (
        CONCATENATING,
        "0x0",
        "Array.prototype.toString = function () { return 'T'; };",
    ),
    (
        CONCATENATING,
        "0x0",
        "Object.prototype.valueOf = function () { return 'V'; };",
    ),
    (
        CONCATENATING,
        "0x0",
        "Array.prototype.join = function () { return 'J'; };",
    ),
    (STRING_INDEX, "0x0", "Object.prototype[20] = 'P';"),
    (
        STRING_MEMBER,
        "0x0",
        "Object.defineProperty(Object.prototype, 'foo', { get: function () { return 'G'; } });",
    ),
    (
        STRING_MEMBER,
        "0x0",
        "function poison(p) { p.foo = 'Q'; } poison(Object.prototype);",
    ),
    (
        STRING_METHOD,
        "0x0",
        "var SP = String.prototype; SP.toUpperCase = function () { return 'U'; };",
    ),
    (ENUMERATING, "0x0", "Object.prototype.zz = 1;"),
    (
        CACHING,
        "0x0",
        "Object.defineProperty(Object.prototype, 'k0', { set: function () {} });",
    ),
    (
        NUMBERING,
        "0x0",
        "Number.prototype.toFixed = function () { return 'N'; };",
    ),
    (
        JOINING,
        "0x0",
        "Object.getPrototypeOf([]).join = function () { return 'J'; };",
    ),
    (
        JOINING,
        "0x0",
        "[].constructor.prototype.join = function () { return 'J'; };",
    ),
    (
        STRING_METHOD,
        "0x0",
        "var defineBuiltIn = function (O, key, value) { O[key] = value; return O; };\ndefineBuiltIn(String.prototype, 'toUpperCase', function () {});",
    ),
];

const KEEPING_WRITES: [(&str, &str); 10] = [
    (
        INDEXING,
        "Array.prototype.flatMap = function () { return []; };",
    ),
    (INDEXING, "Array.prototype[7] = 'patched';"),
    (INDEXING, "delete Object.prototype.toString;"),
    (
        INDEXING,
        "var P = Array.prototype; P.join = function () { return 'J'; };",
    ),
    (INDEXING, "function g(p) { p.x = 1; } g(Array.prototype);"),
    (
        INDEXING,
        "String.prototype.toUpperCase = function () { return 'U'; };",
    ),
    (
        STRING_METHOD,
        "Array.prototype.flatMap = function () { return []; };",
    ),
    (JOINING, "Array.prototype.flatMap = function () { return []; };"),
    (CONCATENATING, "Object.prototype.x = 1;"),
    (
        STRING_METHOD,
        "var defineBuiltIn = function (O, key, value) { O[key] = value; return O; };\ndefineBuiltIn(String.prototype, 'at', function () {});",
    ),
];

#[test]
fn a_prototype_write_to_a_key_the_decode_reads_refuses_the_unit() {
    let wrong: Vec<String> = REFUSING_WRITES
        .iter()
        .filter_map(|(decoder, call, patch)| {
            let (code, notes) = proto_notes(decoder, call, patch);
            let kept = code.contains("__DECODE_0__(");
            let stateful = *decoder == CACHING
                && notes
                    .iter()
                    .any(|note| note.contains("a stateful decoder keeps state"));
            (!(refused(&notes) || stateful) || !kept).then(|| format!("{patch}: {notes:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_realm_value_leaked_to_a_global_leaves_the_decode_as_a_node_safe_bail() {
    let patch = "function leak(O) { keep = O; }\nleak(String.prototype);";
    let (code, _) = proto_notes(STRING_METHOD, "0x0", patch);
    assert!(code.contains("__DECODE_0__("), "{code}");
}

#[test]
fn a_prototype_write_the_decode_never_reads_keeps_the_decodes() {
    let wrong: Vec<String> = KEEPING_WRITES
        .iter()
        .filter_map(|(decoder, patch)| {
            let (code, notes) = proto_notes(decoder, "0x0", patch);
            let expected = if *decoder == STRING_METHOD {
                "console.log(\"LOG\")"
            } else {
                "console.log(\"log\")"
            };
            (refused(&notes) || !code.contains(expected)).then(|| format!("{patch}: {notes:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_decoder_call_that_throws_refuses_the_unit() {
    let throwing =
        "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c].charAt(0); }\n";
    let (code, notes) = proto_notes(throwing, "0x9", "");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("a decoder call threw")),
        "{notes:?}"
    );
    assert!(code.contains("__DECODE_0__(9)"), "{code}");
}

#[test]
fn a_decoder_reading_an_array_hole_bails_whatever_the_prototype_holds() {
    for patch in [
        "Array.prototype[0] = 'X';",
        "Array.prototype.flatMap = function () { return []; };",
    ] {
        let (code, notes) = proto_notes(HOLEY, "0x0", patch);
        assert!(
            notes.iter().any(
                |note| note.starts_with(untwist_deobfuscate::notes::DECODE_BAILED)
                    && note.contains("an array hole")
            ),
            "{patch}: {notes:?}"
        );
        assert!(code.contains("__DECODE_0__(0)"), "{patch}: {code}");
    }
}
