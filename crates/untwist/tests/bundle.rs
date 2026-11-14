mod common;

use common::{CHUNK_A, CHUNK_B, WEBPACK_4};
use untwist::{Options, Pipeline};

#[test]
fn a_detected_bundle_is_exposed_on_the_output() {
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let bundle = out.bundle.as_ref().expect("bundle");
    assert_eq!(bundle.kind, "webpack");
    assert_eq!(bundle.entry_id.as_deref(), Some("0"));
    assert_eq!(
        bundle
            .modules
            .iter()
            .map(|module| (module.id.as_str(), module.path.as_str(), module.is_entry))
            .collect::<Vec<_>>(),
        vec![("0", "index.js", true), ("1", "lib.js", false)]
    );
    assert_eq!(
        bundle.entry().map(|module| module.code.trim_end()),
        Some("const lib = require(\"./lib.js\");\nconsole.log(lib);")
    );
}

#[test]
fn no_unpack_leaves_the_bundle_unset() {
    let out = Pipeline::new(Options {
        unpack: false,
        ..Options::default()
    })
    .run(WEBPACK_4);
    assert!(out.bundle.is_none());
    assert!(!out.stages.iter().any(|stage| stage.stage == "unpack"));
}

#[test]
fn files_carry_bundle_json_and_one_file_per_module() {
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let bundle = out.bundle.as_ref().expect("bundle");
    let files = bundle.files();
    assert_eq!(
        files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["bundle.json", "index.js", "lib.js"]
    );
    let json: serde_json::Value = serde_json::from_str(&files[0].code).expect("valid json");
    assert_eq!(json["type"], "webpack");
    assert_eq!(json["entryId"], "0");
    assert_eq!(json["modules"][0]["path"], "index.js");
}

#[test]
fn a_program_with_no_bundle_still_reports_the_stage() {
    let out = Pipeline::new(Options::default()).run("console.log(1);");
    assert!(out.bundle.is_none());
    assert!(out.stages.iter().any(|stage| stage.stage == "unpack"));
}

#[test]
fn chunk_files_merge_into_one_bundle() {
    let out = Pipeline::new(Options::default()).run_many(&[CHUNK_A, CHUNK_B]);
    let bundle = out.bundle.as_ref().expect("bundle");
    assert_eq!(bundle.chunk_ids, vec!["15".to_string(), "16".to_string()]);
    assert_eq!(
        bundle
            .modules
            .iter()
            .map(|module| module.id.as_str())
            .collect::<Vec<_>>(),
        vec!["2wwy", "E8gZ", "w6GO"]
    );
    assert_eq!(
        bundle.modules[0].code.trim_end(),
        "const a = require(\"./E8gZ.js\");\nconst b = require(\"./w6GO.js\");"
    );
    assert!(
        !out.notes
            .iter()
            .any(|note| note.message.contains("no module in this bundle")),
        "{:?}",
        out.notes
    );
    assert_eq!(out.codes.len(), 2);
}

#[test]
fn a_single_chunk_still_reports_the_missing_target() {
    let out = Pipeline::new(Options::default()).run(CHUNK_A);
    assert!(out.notes.iter().any(|note| note
        .message
        .contains("require(\"w6GO\") has no module in this bundle")));
}

const A_SURVIVING_CONTROLLER: &str = "var ctrl = function () {
  var f = !![];
  return function (c, fn) {
    var r = f ? function () {
      if (fn) {
        var x = fn.apply(c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = ![];
    return r;
  };
}();";

const THE_CONTROLLER_ON_ONE_LINE: &str =
    "function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}}";

fn without_deobfuscation(source: &str) -> untwist::Output {
    Pipeline::new(Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    })
    .run(source)
}

fn noted<'o>(out: &'o untwist::Output, message: &str) -> Vec<&'o str> {
    out.notes
        .iter()
        .map(|note| note.message.as_str())
        .filter(|text| text.starts_with(message))
        .collect()
}

#[test]
fn a_guard_inside_a_bundle_module_is_printed_on_one_line_and_noted_once() {
    let source = WEBPACK_4.replace("e.exports = 1;", A_SURVIVING_CONTROLLER);
    let out = without_deobfuscation(&source);
    let bundle = out.bundle.as_ref().expect("bundle");
    let module = bundle
        .modules
        .iter()
        .find(|module| module.code.contains("var ctrl"))
        .expect("the controller's module");
    assert!(
        module.code.contains(THE_CONTROLLER_ON_ONE_LINE),
        "{}",
        module.code
    );
    assert!(
        out.code().contains(THE_CONTROLLER_ON_ONE_LINE),
        "{}",
        out.code()
    );
    let expected = format!("{} 1", untwist::GUARD_ON_ONE_LINE);
    assert_eq!(
        noted(&out, untwist::GUARD_ON_ONE_LINE),
        vec![expected.as_str()]
    );
}

#[test]
fn a_guard_kept_across_lines_inside_a_bundle_module_is_noted_once() {
    let kept =
        A_SURVIVING_CONTROLLER.replace("fn.apply(c, arguments)", "fn[`ap\nply`](c, arguments)");
    let out = without_deobfuscation(&WEBPACK_4.replace("e.exports = 1;", &kept));
    let expected = format!("{} 1", untwist::GUARD_KEPT_ACROSS_LINES);
    assert_eq!(
        noted(&out, untwist::GUARD_KEPT_ACROSS_LINES),
        vec![expected.as_str()]
    );
    assert!(noted(&out, untwist::GUARD_ON_ONE_LINE).is_empty());
}

#[test]
fn two_bundles_holding_the_same_guard_note_it_once_per_input() {
    let source = WEBPACK_4.replace("e.exports = 1;", A_SURVIVING_CONTROLLER);
    let out = Pipeline::new(Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    })
    .run_many(&[&source, &source]);
    let expected = format!("{} 1", untwist::GUARD_ON_ONE_LINE);
    assert_eq!(
        noted(&out, untwist::GUARD_ON_ONE_LINE),
        vec![expected.as_str(); 2]
    );
}

const A_HELPER_READING_A_MODULE_PARAMETER: &str = "!(function(e){var a={};function n(t){if(a[t])return a[t].exports;var r=(a[t]={exports:{}});return e[t].call(r.exports,r,r.exports,n),r.exports}return n((n.s=0))})([function(e,t,i){const lib=i(1);i(2);console.log(lib)},function(e,t,i){var F=function(s){var A='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var a='';var b='';var c=a+i;for(var q=0,t,n,j=0;n=s['charAt'](j++);~n&&(t=q%4?t*64+n:n,q++%4)?a+=c['charCodeAt'](j+10)-10!==0?String['fromCharCode'](255&t>>(-2*q&6)):q:0){n=A['indexOf'](n)}return a};console.log(F('aGVsbG8gd29ybGQ='));e.exports=1},function(e,t,i){e.exports=2}]);";

#[test]
fn a_guard_whose_text_unpack_renames_is_noted_once() {
    let out = without_deobfuscation(A_HELPER_READING_A_MODULE_PARAMETER);
    let expected = format!("{} 1", untwist::GUARD_ON_ONE_LINE);
    assert_eq!(
        noted(&out, untwist::GUARD_ON_ONE_LINE),
        vec![expected.as_str()]
    );
}
