use untwist::{Options, Pipeline};

const UNPROVEN: &str = "a stateful decoder keeps state that is not a proven memo or init gate";

const FIXTURES: [&str; 3] = [
    "obfuscator.io-memo-reset-base64.js",
    "obfuscator.io-memo-reset-rc4.js",
    "obfuscator.io-memo-reset-protections.js",
];

const BASE64: &str = "obfuscator.io-memo-reset-base64.js";

const DECODER: &str = "_0x1ab0(";

fn run(source: &str, unsafe_transforms: bool) -> (String, Vec<String>) {
    let options = Options {
        unpack: false,
        jsx: false,
        unsafe_transforms,
        extended: false,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(source);
    let notes = out.notes.iter().map(|note| note.message.clone()).collect();
    (out.code().to_string(), notes)
}

fn bailed(notes: &[String]) -> bool {
    notes.iter().any(|note| note.contains("decoding bailed"))
}

#[test]
fn every_memo_reset_fixture_decodes_in_both_lanes() {
    for name in FIXTURES {
        let source = untwist_testing::read_fixture("deobfuscate", name);
        for unsafe_transforms in [false, true] {
            let (code, notes) = run(&source, unsafe_transforms);
            assert!(
                code.contains("console.log(\"Hello World!\")") && !bailed(&notes),
                "{name}, unsafe {unsafe_transforms}: {notes:?}\n{code}"
            );
        }
    }
}

#[test]
fn the_obfuscator_io_site_sample_decodes_in_both_lanes() {
    let source = untwist_testing::read_fixture("deobfuscate", "obfuscator.io-site-memo-reset.js");
    for unsafe_transforms in [false, true] {
        let (code, notes) = run(&source, unsafe_transforms);
        assert!(
            code.contains("console.log(\"\")") && !bailed(&notes),
            "unsafe {unsafe_transforms}: {notes:?}\n{code}"
        );
    }
}

const SABOTAGE: &[(&str, &str, &str)] = &[
    (
        "a third write in the gate",
        "_0x1ab0['NWedsi']=_0x1b9a8c);",
        "_0x1ab0['NWedsi']=_0x1b9a8c,_0x1ab0['xKqPzm']=0x1);",
    ),
    (
        "a memo reset to a non-empty object",
        "&&(_0x1ab0['nfVyLk']={},",
        "&&(_0x1ab0['nfVyLk']={'0':'forged'},",
    ),
    (
        "a gate on the second array element",
        "var _0x1b9a8c=_0x2a67d5[0x0];",
        "var _0x1b9a8c=_0x2a67d5[0x1];",
    ),
    (
        "a gate on another array",
        "var _0x1b9a8c=_0x2a67d5[0x0];",
        "var _0x4d2e11=['Bg9N'];var _0x1b9a8c=_0x4d2e11[0x0];",
    ),
    (
        "a state read outside the gate",
        "return _0x11e7cd===undefined",
        "return _0x1ab0['NWedsi'],_0x11e7cd===undefined",
    ),
    (
        "a gate value that escapes into the result",
        "_0x1ab0cd=_0x11e7cd,_0x1ab0cd;}",
        "_0x1ab0cd=_0x11e7cd,_0x1ab0cd+_0x1b9a8c['slice'](0x63);}",
    ),
    (
        "a memo read outside its lookup",
        "var _0x11e7cd=_0x1ab0['nfVyLk'][_0x308517];",
        "var _0x11e7cd=_0x1ab0['nfVyLk'][_0x308517];var _0x2f19ab=_0x1ab0['nfVyLk'];",
    ),
];

#[test]
fn a_sabotaged_memo_reset_gate_in_a_generated_fixture_is_refused() {
    let source = untwist_testing::read_fixture("deobfuscate", BASE64);
    for (row, from, to) in SABOTAGE {
        assert_eq!(source.matches(from).count(), 1, "{row}: {from}");
        let sabotaged = source.replacen(from, to, 1);
        for unsafe_transforms in [false, true] {
            let (code, notes) = run(&sabotaged, unsafe_transforms);
            assert!(
                code.contains(DECODER) && notes.iter().any(|note| note.contains(UNPROVEN)),
                "{row}, unsafe {unsafe_transforms}: {notes:?}\n{code}"
            );
        }
    }
}
