mod common;

use common::cpu_or_wall;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::unpack;

const BUNDLERS: &str = "../../fixtures";
const PARCEL: &str = "unpack-bundlers/parcel/min-parcel-hoist.js";
const PARCEL_FIRST_REGISTER: &str = "parcelRegister(\"d4t4A\"";
const METRO: &str = "unpack-bundlers/metro/metro.js";
const SMALL: usize = 4_000;
const LARGE: usize = 32_000;
const LINEAR_FACTOR: u32 = 3;
const SLACK_MS: u64 = 100;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(BUNDLERS)
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

fn parcel_helpers(count: usize) -> String {
    let fixture = fixture(PARCEL);
    let head = fixture
        .split(PARCEL_FIRST_REGISTER)
        .next()
        .expect("a hoisted prelude");
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!(
            "var h{index} = function () {{ return {index}; }};\n"
        ));
    }
    source.push_str(head);
    for index in 0..count {
        source.push_str(&format!(
            "parcelRegister(\"m{index}\", function (module, exports) {{ module.exports = h{index}(); }});\n"
        ));
    }
    source.push_str("console.log(parcelRequire(\"m0\"));\n");
    source
}

fn metro_prelude(count: usize) -> String {
    let bundle = fixture(METRO);
    let end = bundle.find("\n__d(").expect("a first definition") + 1;
    let mut source = bundle[..end].to_string();
    for index in 0..count {
        source.push_str(&format!("var v{index} = {{}};\n"));
    }
    for index in 0..count {
        source.push_str(&format!(
            "__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {{ exports.x = Math.max({index}, 1) + v{index}.length; }}, {index}, []);\n"
        ));
    }
    source.push_str("__r(0);\n");
    source
}

fn unpack_time(source: &str, modules: usize) -> std::time::Duration {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let (found, spent) = cpu_or_wall(|| {
        unpack(&alloc, &parsed.program, &[]).map(|result| result.bundle.modules.len())
    });
    assert!(
        found.is_some_and(|found| found > modules),
        "{modules} modules: {found:?}"
    );
    spent
}

fn grows_linearly(bundle: fn(usize) -> String) {
    let small = unpack_time(&bundle(SMALL), SMALL);
    let large = unpack_time(&bundle(LARGE), LARGE);
    let bound =
        small * (LARGE / SMALL) as u32 * LINEAR_FACTOR + std::time::Duration::from_millis(SLACK_MS);
    assert!(
        large < bound,
        "{LARGE} took {large:?}, {SMALL} took {small:?}"
    );
}

#[test]
fn parcel_link_time_grows_linearly_with_modules_and_hoisted_helpers() {
    grows_linearly(parcel_helpers);
}

#[test]
fn metro_link_time_grows_linearly_with_modules_and_prelude_vars() {
    grows_linearly(metro_prelude);
}
