use untwist::{Options, Pipeline};
use untwist_core::semantic::{assume_observers, with_observes, Observers};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const HEADER: &str = "\
function sa() { var h = ['log', 'hello', 'world']; sa = function () { return h; }; return sa(); }
function dec(c, d) { var e = sa(); return (dec = function (f, g) { f = f - 0x0; var i = e[f];
  return String.fromCharCode(i.charCodeAt(0)) + i.slice(1); }), dec(c, d); }
var E = function () { return 'X'; };
";

const TAIL: &str = "console.log(dec(0x1), dec(0x2));";

const DECODED: &str = "(\"hello\", \"world\")";

const WITH_SCOPES: [(&str, &str); 5] = [
    (
        "an object key named like the decoder",
        "var obj = { dec: E }; with (obj) { console.log(dec(0x1), dec(0x2)); }",
    ),
    (
        "a stored decoder alias",
        "var d = dec; with ({ d: E }) { console.log(d(0x1), d(0x2)); }",
    ),
    (
        "a stored decoder wrapper",
        "function w(a) { return dec(a); } with ({ w: E }) { console.log(w(0x1), w(0x2)); }",
    ),
    (
        "a stored decoder member",
        "var o = { k: dec }; with ({ o: { k: E } }) { console.log(o.k(0x1), o.k(0x2)); }",
    ),
    (
        "the global object",
        "with (globalThis) { console.log(dec(0x1), dec(0x2)); }",
    ),
];

const SPELLED_NAMES: [(&str, &str); 6] = [
    (
        "the decoder through globalThis",
        "globalThis[\"d\" + \"ec\"] = E;",
    ),
    ("the decoder through this", "this[\"d\" + \"ec\"] = E;"),
    (
        "the decoder through an eval of a joined name",
        "var n = \"d\" + \"ec\"; eval(n + \" = E\");",
    ),
    (
        "the decoder through an eval of joined text",
        "eval(\"d\" + \"ec = E\");",
    ),
    (
        "the decoder through an indirect eval",
        "(0, eval)(\"d\" + \"ec = E\");",
    ),
    (
        "the string array through globalThis",
        "globalThis[\"s\" + \"a\"] = function () { return [\"X\", \"X\", \"X\"]; };",
    ),
];

fn program(patch: &str) -> String {
    format!("{HEADER}{patch}\n{TAIL}\n")
}

fn safe(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn decoded_rows(rows: &[(&str, &str)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, patch)| safe(&program(patch)).contains(DECODED))
        .map(|(label, _)| label.to_string())
        .collect()
}

fn printed_like_node(rows: &[(&str, &str)]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run_safe = |source: &str| safe(source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let programs: Vec<(String, String)> = rows
        .iter()
        .map(|(label, patch)| (label.to_string(), program(patch)))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_lane_but_the_assumed_one_observes_with() {
    for observers in [Observers::Every, Observers::Open] {
        let _guard = assume_observers(observers);
        assert!(with_observes(), "{observers:?}");
    }
    let _guard = assume_observers(Observers::None);
    assert!(!with_observes());
}

#[test]
fn the_default_lane_is_a_lane_that_observes_with() {
    let options = Options::default();
    assert!(options.extended && !options.unsafe_transforms);
    let output = Pipeline::new(options).run(&program("with ({}) {}"));
    assert!(!output.code().contains(DECODED), "{}", output.code());
    assert!(
        output
            .notes
            .iter()
            .any(|note| note.message.contains("with statement")),
        "{:?}",
        output.notes
    );
}

#[test]
fn a_with_scope_around_a_stored_decoder_refuses_the_decode() {
    let decoded = decoded_rows(&WITH_SCOPES);
    assert!(decoded.is_empty(), "decoded anyway: {decoded:?}");
}

#[test]
fn a_with_scope_around_a_stored_decoder_prints_what_node_prints() {
    printed_like_node(&WITH_SCOPES);
}

#[test]
fn a_machinery_name_joined_from_pieces_refuses_the_decode() {
    let decoded = decoded_rows(&SPELLED_NAMES);
    assert!(decoded.is_empty(), "decoded anyway: {decoded:?}");
}

#[test]
fn a_machinery_name_joined_from_pieces_prints_what_node_prints() {
    printed_like_node(&SPELLED_NAMES);
}
