mod common;

const OUT_OF_RANGE: &str = "\
function a() { var h = ['aGVsbG8=']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var i = e[f]; return i.charAt(0); }; return b(c, d); }
console.log(b(0x0), b(0x5));";

const VOID_SLOT: &str = "\
function a() { var h = ['aGVsbG8=', void 0]; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var i = e[f]; return i.charAt(0); }; return b(c, d); }
console.log(b(0x0), b(0x1));";

const SHORT_WRAPPER: &str = "\
function a() { var h = ['aGVsbG8=']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f] + g.charAt(0); }; return b(c, d); }
function w(x) { return b(x); }
console.log(w(0x0));";

#[test]
fn issue_113_no_variant_panics_and_the_good_site_still_decodes() {
    let (code, notes) = common::run_passes(OUT_OF_RANGE);
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(!code.contains("\"a\""), "{code}");
    assert!(
        notes.iter().any(|note| note.contains("decoding bailed")),
        "{notes:?}"
    );

    let (code, notes) = common::run_passes(VOID_SLOT);
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(
        notes.iter().any(|note| note.contains("decoding bailed")),
        "{notes:?}"
    );

    let (code, _notes) = common::run_passes(SHORT_WRAPPER);
    assert!(code.contains("function w(x)"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(code.contains("function b(c, d)"), "{code}");
    assert!(code.contains("w(0)"), "{code}");
}

#[test]
fn issue_113_the_whole_pipeline_survives_every_variant() {
    for source in [OUT_OF_RANGE, VOID_SLOT, SHORT_WRAPPER] {
        let out = untwist::Pipeline::new(untwist::Options::default()).run(source);
        assert!(
            out.stages
                .iter()
                .all(|stage| stage.stop == untwist::StopReason::Converged),
            "{source}"
        );
    }
}
