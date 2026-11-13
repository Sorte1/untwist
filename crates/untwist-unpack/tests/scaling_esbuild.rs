mod common;

use common::cpu_or_wall;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::unpack;

const DIRECT: &str = "var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
";

const RUNS: usize = 3;

fn fastest<T>(mut work: impl FnMut() -> T) -> (T, std::time::Duration) {
    let (mut kept, mut best) = cpu_or_wall(&mut work);
    for _ in 1..RUNS {
        let (done, spent) = cpu_or_wall(&mut work);
        if spent < best {
            (kept, best) = (done, spent);
        }
    }
    (kept, best)
}

fn hostile_bundle(modules: usize) -> String {
    let separate = modules / 4;
    let mut source = String::new();
    for index in 0..modules {
        source.push_str(&format!("// head {index}\n"));
    }
    source.push_str(DIRECT);
    for index in 0..modules {
        source.push_str(&format!("var pure{index} = Object.keys;\n"));
    }
    for index in 0..separate {
        source.push_str(&format!(
            "// m{index}.js\nvar require_m{index} = __commonJS((exports, module) => {{ module.exports = pure{index} && {index}; }});\n"
        ));
    }
    let shared: Vec<String> = (separate..modules)
        .map(|index| {
            format!("require_m{index} = __commonJS((exports, module) => {{ module.exports = pure{index} && {index}; }})")
        })
        .collect();
    source.push_str(&format!("var {}, extra = 1;\n", shared.join(",\n  ")));
    source.push_str(&format!(
        "console.log(require_m0(), require_m{}(), extra);\n",
        modules - 1
    ));
    for index in 0..modules {
        source.push_str(&format!("// tail {index}\n"));
    }
    source
}

#[test]
fn a_hostile_hundred_thousand_module_bundle_splits_in_linear_time() {
    const MODULES: usize = 100_000;
    const LINEAR_FACTOR: u32 = 5;
    let source = hostile_bundle(MODULES);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let (_, parse) = fastest(|| {
        let alloc = Allocator::default();
        parse_program(&alloc, &source).program.body.len()
    });
    let (_, scope) = fastest(|| untwist_core::semantic::build_scoping(&parsed.program));
    let (modules, spent) =
        fastest(|| unpack(&alloc, &parsed.program, &[]).map(|result| result.bundle.modules.len()));
    assert_eq!(modules, Some(MODULES + 2));
    let bound = (parse + scope) * LINEAR_FACTOR;
    assert!(
        spent < bound,
        "{MODULES} modules took {spent:?}, over {LINEAR_FACTOR} x parse {parse:?} + scoping {scope:?}"
    );
}
