use oxc_allocator::Allocator;
use untwist::libraries::{self, LibraryTables};
use untwist::{KeptReason, Options, Output, Pipeline};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::UnitKind;
use untwist_rename::library::table::{exact_key, loose_key, Role, RowData, Table};
use untwist_rename::library::unit_records;

const BUNDLE: &str = "!(function (e) {
  var a = {};
  function n(t) {
    if (a[t]) return a[t].exports;
    var r = (a[t] = { exports: {} });
    return e[t].call(r.exports, r, r.exports, n), r.exports;
  }
  return n((n.s = 0));
})([
  function (e, t, i) {
    const pad = i(1);
    const shout = i(2);
    console.log(pad('7', 3, '0'), shout('hi'));
  },
  function (e, t, i) {
    e.exports = function pad(text, width, fill) {
      var out = String(text);
      while (out.length < width) {
        out = (fill || ' ') + out;
      }
      return out;
    };
  },
  function (e, t, i) {
    const tail = i(3);
    e.exports = function shout(text) {
      return String(text).toUpperCase() + tail.mark + tail.mark;
    };
  },
  function (e, t, i) {
    e.exports = { mark: '!', count: function (text) { return String(text).split('!').length - 1; } };
  },
]);
";

fn run(imports: bool) -> Output {
    Pipeline::new(Options {
        library_imports: imports,
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(BUNDLE)
}

fn program_row(code: &str, label: &str, module: &str, specifier: &str) -> RowData {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, code);
    let sem = build_scoping(&parsed.program);
    let record = unit_records(&parsed.program, &sem.scoping)
        .into_iter()
        .find(|record| record.kind == UnitKind::Program)
        .expect("a program unit");
    RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: label.to_string(),
        versions: "1.0.0~1.2.0".to_string(),
        module: module.to_string(),
        role: Role::Complete,
        own: Some(specifier.to_string()),
        inner: Vec::new(),
        outer: Vec::new(),
        specific: record.specific,
    }
}

fn module_code(output: &Output, index: usize) -> String {
    output.bundle.as_ref().expect("a bundle").modules[index]
        .code
        .clone()
}

#[test]
fn a_whole_module_match_becomes_a_require_of_the_pinned_package() {
    let plain = run(false);
    assert!(plain.library_imports.is_none());
    let bundle = plain.bundle.as_ref().expect("the bundle unpacks");
    assert_eq!(bundle.modules.len(), 4);
    let rows = vec![
        program_row(
            &module_code(&plain, 1),
            "left-pad-ish",
            "index.js",
            "left-pad-ish",
        ),
        program_row(
            &module_code(&plain, 2),
            "shouty",
            "lib/shout.js",
            "shouty/lib/shout.js",
        ),
    ];
    let text = Table::from_rows(&rows).write("imports-test");
    libraries::load(LibraryTables::from_text(&text).expect("the test table"));

    let output = run(true);
    let report = output.library_imports.clone().expect("an imports report");
    let pad = module_code(&output, 1);
    assert!(
        pad.contains("module.exports = require(\"left-pad-ish\")"),
        "{pad}"
    );
    assert_eq!(
        report.pins.get("left-pad-ish").map(String::as_str),
        Some("1.2.0")
    );
    assert_eq!(report.replaced.len(), 1, "{report:?}");
    assert!(!module_code(&output, 2).contains("require(\"shouty"));
    assert_eq!(report.kept.len(), 1, "{report:?}");
    assert_eq!(report.kept[0].package, "shouty");
    assert_eq!(report.kept[0].reason, KeptReason::SideEffects);
    let manifest = report.package_json().expect("a package.json");
    assert!(
        manifest.contains("\"left-pad-ish\": \"1.2.0\""),
        "{manifest}"
    );
    assert!(!manifest.contains("shouty"), "{manifest}");
    assert_eq!(module_code(&output, 0), module_code(&plain, 0));

    let dir = tempfile::tempdir().expect("a temp dir");
    output.write_to(dir.path()).expect("the tree is written");
    let written = std::fs::read_to_string(dir.path().join("package.json")).expect("package.json");
    assert_eq!(written, manifest);
}
