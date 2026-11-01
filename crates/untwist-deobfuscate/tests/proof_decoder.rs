use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::bridge::build_requests;
use untwist_deobfuscate::notes;

const HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
";

const READER: &str = "\
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { f = f - 0x0; return e[f]; }; return __DECODE_0__(c, d); }
";

const CHARCODE_DECODER: &str = "\
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return String.fromCharCode(e[c].charCodeAt(0)); }
";

fn first_refusal(source: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let plan = build_requests(&parsed.program, &semantics.scoping);
    plan.refusals.first().map(|refusal| refusal.note)
}

fn with_calls(decoder: &str, rest: &str) -> String {
    format!("{HEADER}{decoder}console[__DECODE_0__(0x0)](__DECODE_0__(0x1));\n{rest}")
}

#[test]
fn a_reading_decoder_is_decoded() {
    assert_eq!(first_refusal(&with_calls(READER, "")), None);
}

#[test]
fn a_decoder_that_rotates_returns_or_writes_the_array_is_refused() {
    let rotating = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); e.push(e.shift()); return e[c]; }\n";
    let returning = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); if (c === -1) return e; return e[c]; }\n";
    let writing =
        "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); e[0] = 'y'; return e[c]; }\n";
    for decoder in [rotating, returning, writing] {
        let refusal = first_refusal(&with_calls(decoder, ""));
        assert_eq!(refusal, Some(notes::DECODER_NOT_READ_ONLY), "{decoder}");
    }
}

#[test]
fn a_decoder_that_reads_outer_state_is_refused() {
    let decoder =
        "var n = 0;\nfunction __DECODE_0__(c) { var e = __STRING_ARRAY__(); n++; return e[c]; }\n";
    let refusal = first_refusal(&with_calls(decoder, ""));
    assert_eq!(refusal, Some(notes::DECODER_READS_OUTER_BINDING));
}

#[test]
fn a_used_built_in_replaced_anywhere_refuses_the_unit() {
    for patch in [
        "String.fromCharCode = function () { return 'P'; };",
        "Object.defineProperty(String, 'fromCharCode', { value: 1 });",
        "window.String = null;",
        "String = null;",
    ] {
        let refusal = first_refusal(&with_calls(CHARCODE_DECODER, patch));
        assert_eq!(refusal, Some(notes::BUILTIN_PATCHED), "{patch}");
    }
}

#[test]
fn a_used_built_in_replaced_via_its_prototype_refuses_the_unit_after_decode() {
    for patch in [
        "String.prototype.charCodeAt = function () { return 80; };",
        "delete String.prototype.charCodeAt;",
    ] {
        let refusal = first_refusal(&with_calls(CHARCODE_DECODER, patch));
        assert_eq!(refusal, Some(notes::BUILTIN_PATCHED), "{patch}");
    }
}

#[test]
fn an_unused_built_in_replaced_or_a_shadowing_local_does_not_refuse() {
    for patch in [
        "Number.prototype.round = function () {};",
        "window.saveVal = function () {};",
        "function f() { var String = {}; String.fromCharCode = 1; }",
    ] {
        assert_eq!(
            first_refusal(&with_calls(CHARCODE_DECODER, patch)),
            None,
            "{patch}"
        );
    }
}

#[test]
fn a_prototype_write_alone_is_judged_after_the_decode() {
    for patch in [
        "Array.prototype.flatMap = function () {};",
        "Array.prototype[7] = 'patched';",
        "Object.prototype.x = 1;",
        "delete Object.prototype.toString;",
        "Object.defineProperty(Array.prototype, '7', { get: function () { return 'g'; } });",
        "Object.setPrototypeOf(Array.prototype, null);",
        "[].__proto__.x = 1;",
        "var P = Array.prototype; P.join = function () {};",
        "function g(p) { p[7] = 'x'; } g(Array.prototype);",
    ] {
        assert_eq!(first_refusal(&with_calls(READER, patch)), None, "{patch}");
    }
}

#[test]
fn a_decoder_that_repoints_the_array_prototype_is_refused() {
    for decoder in [
        "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); e.__proto__ = null; return e[c]; }\n",
        "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); Object.setPrototypeOf(e, null); return e[c]; }\n",
    ] {
        let refusal = first_refusal(&with_calls(decoder, ""));
        assert_eq!(refusal, Some(notes::DECODER_NOT_READ_ONLY), "{decoder}");
    }
}

#[test]
fn a_built_in_refusal_names_the_property_written() {
    let alloc = Allocator::default();
    for (patch, named) in [
        (
            "String.fromCharCode = function () { return 'P'; };",
            "String.fromCharCode",
        ),
        ("window.String = null;", "window.String"),
    ] {
        let source = with_calls(CHARCODE_DECODER, patch);
        let parsed = parse_program(&alloc, &source);
        let semantics = build_scoping(&parsed.program);
        let plan = build_requests(&parsed.program, &semantics.scoping);
        let message = plan.refusals.first().map(|refusal| refusal.message());
        assert_eq!(
            message,
            Some(format!("{}: {named}", notes::BUILTIN_PATCHED)),
            "{patch}"
        );
    }
}
