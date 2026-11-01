use untwist::{Options, Output, Pipeline, RenameMode};

const OBFUSCATED_WEBPACK_4: &str = "function a() { var h = ['call', 'exports', 'log', 'hello from module', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); return (b = function (f, g) { f = f - 0; var i = e[f]; return i; }), b(c, d); }
!(function (e) {
  var r = {};
  function n(t) {
    if (r[t]) return r[t][b(1)];
    var m = (r[t] = { i: t, l: false, exports: {} });
    return e[t][b(0)](m[b(1)], m, m[b(1)], n), (m.l = true), m[b(1)];
  }
  n.m = e;
  n.c = r;
  return n((n.s = 0));
})([
  function (e, t, i) { var lib = i(1); console[b(2)](b(3), lib); },
  function (e, t, i) { e[b(1)] = b(4); },
]);
";

const SHADOWED_REQUIRE: &str = "!(function (e) {
  var a = {};
  function n(t) {
    if (a[t]) return a[t].exports;
    var r = (a[t] = { exports: {} });
    return e[t].call(r.exports, r, r.exports, n), r.exports;
  }
  n.r = function (t) { Object.defineProperty(t, '__esModule', { value: true }); };
  n.d = function (t, k, g) { Object.defineProperty(t, k, { enumerable: true, get: g }); };
  return n((n.s = 0));
})([
  function (e, t, i) {
    'use strict';
    i.r(t);
    var lib = i(1);
    for (const i of [{ code: 1 }]) console.log(i.code, lib.default);
    switch (lib.default) { case 2: let i = { usedTimes: 3 }; console.log(i.usedTimes); }
  },
  function (e, t, i) {
    'use strict';
    i.r(t);
    i.d(t, 'default', function () { return 2; });
  },
]);
";

const RENAMES: [RenameMode; 4] = [
    RenameMode::None,
    RenameMode::Mangle,
    RenameMode::Simple,
    RenameMode::Unique,
];

fn run(unsafe_transforms: bool, rename: RenameMode, unpack: bool) -> Output {
    Pipeline::new(Options {
        unsafe_transforms,
        rename,
        unpack,
        jsx: false,
        ..Options::default()
    })
    .run(OBFUSCATED_WEBPACK_4)
}

fn text(out: &Output) -> String {
    match &out.bundle {
        Some(bundle) => bundle
            .modules
            .iter()
            .map(|module| module.code.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        None => out.code().to_string(),
    }
}

fn decoded(out: &Output) -> bool {
    text(out).contains("\"hello from module\"")
}

fn modules(out: &Output) -> usize {
    out.bundle.as_ref().map_or(0, |bundle| bundle.modules.len())
}

#[test]
fn a_loop_or_case_binding_that_shadows_require_does_not_block_unpacking() {
    let out = Pipeline::new(Options::default()).run(SHADOWED_REQUIRE);
    assert_eq!(modules(&out), 3, "{}", out.code());
}

#[test]
fn unsafe_decodes_and_unpacks_the_obfuscated_bootstrap_in_every_rename_and_output_mode() {
    for rename in RENAMES {
        for unpack in [false, true] {
            let out = run(true, rename, unpack);
            assert!(decoded(&out), "{rename:?} unpack {unpack}: {}", text(&out));
            if unpack {
                assert_eq!(modules(&out), 2, "{rename:?}: {}", out.code());
            }
        }
    }
}

#[test]
fn the_safe_lane_decodes_and_unpacks_the_obfuscated_bootstrap_in_every_rename_and_output_mode() {
    for rename in RENAMES {
        for unpack in [false, true] {
            let out = run(false, rename, unpack);
            assert!(decoded(&out), "{rename:?} unpack {unpack}: {}", text(&out));
            if unpack {
                assert_eq!(modules(&out), 2, "{rename:?}: {}", out.code());
            }
        }
    }
}

#[test]
fn the_decode_decision_does_not_depend_on_rename_or_output_mode() {
    for unsafe_transforms in [false, true] {
        let want = decoded(&run(unsafe_transforms, RenameMode::None, false));
        for rename in RENAMES {
            for unpack in [false, true] {
                let out = run(unsafe_transforms, rename, unpack);
                assert_eq!(
                    decoded(&out),
                    want,
                    "unsafe {unsafe_transforms} {rename:?} unpack {unpack}"
                );
            }
        }
    }
}

const LOCAL_SAMPLES: [(&str, usize); 3] = [
    ("game_old.js", 381),
    (
        "export/game-312f4d2ad7c6f56271b952651140ff4b5d3fe3d880270afd98093b1db7f9ab7f-7.1.8.js",
        382,
    ),
    ("game.js", 0),
];

const OUTPUT_MODES: [(&str, bool, bool); 3] = [
    ("stdout", false, false),
    ("single-file", true, true),
    ("modules", true, false),
];

fn undecoded_references(out: &Output) -> usize {
    out.notes
        .iter()
        .filter_map(|note| {
            note.message
                .strip_prefix("decoder machinery kept: ")?
                .split_once(" reference(s)")?
                .0
                .parse::<usize>()
                .ok()
        })
        .sum()
}

fn run_file(
    source: &str,
    unsafe_transforms: bool,
    rename: RenameMode,
    mode: (bool, bool),
) -> Output {
    Pipeline::new(Options {
        unsafe_transforms,
        rename,
        unpack: mode.0,
        single_file: mode.1,
        ..Options::default()
    })
    .run(source)
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "user files; run with cargo test --release -p untwist --test rename_independent_decode"
)]
fn user_files_decode_alike_in_every_rename_and_output_mode() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, reference_modules) in LOCAL_SAMPLES {
        let Ok(source) = std::fs::read_to_string(root.join(name)) else {
            eprintln!("{name} is absent; skipped");
            continue;
        };
        for unsafe_transforms in [false, true] {
            let mut seen = Vec::new();
            for rename in RENAMES {
                for (mode, unpack, single) in OUTPUT_MODES {
                    let out = run_file(&source, unsafe_transforms, rename, (unpack, single));
                    let left = undecoded_references(&out);
                    eprintln!(
                        "{name} unsafe {unsafe_transforms} {rename:?} {mode}: {left} left, {} modules",
                        modules(&out)
                    );
                    if unsafe_transforms && reference_modules > 0 {
                        assert_eq!(left, 0, "{name} {rename:?} {mode}");
                        if unpack {
                            assert!(
                                modules(&out) >= reference_modules,
                                "{name} {rename:?} {mode}"
                            );
                        }
                    }
                    seen.push(left);
                }
            }
            assert!(
                seen.windows(2).all(|pair| pair[0] == pair[1]),
                "{name} unsafe {unsafe_transforms}: {seen:?}"
            );
        }
    }
}
