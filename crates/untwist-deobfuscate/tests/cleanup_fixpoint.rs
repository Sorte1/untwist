mod common;

#[test]
fn split_strings_are_merged_after_decoding() {
    let (code, _notes) = common::run_passes(
        "\
function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0x0) + b(0x1));",
    );
    assert_eq!(code.trim(), "console.log(\"Hello\");");
}

const A_COMPUTED_OBJECT_READ_THAT_ONLY_DECODES_LATE: &str = "\
function a() { var h = ['alpha', 'log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var m = { alpha: 1, beta: 2 };
console[b(0x1)](m[b(0x0)]);";

#[test]
fn an_object_read_whose_key_decodes_late_is_inlined_after_the_machinery_goes() {
    let (code, _notes) = common::run_passes(A_COMPUTED_OBJECT_READ_THAT_ONLY_DECODES_LATE);
    assert_eq!(code.trim(), "console[\"log\"](1);");
    assert!(!code.contains('m'), "{code}");
}
