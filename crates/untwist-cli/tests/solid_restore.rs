mod common;

use common::bin;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist-solid/tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).expect("the fixture reads")
}

fn code(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("deobfuscated.js")).unwrap()
}

#[test]
fn restore_solid_writes_the_component_beside_the_unchanged_code() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Counter.js");
    std::fs::write(&input, fixture("Counter.js")).unwrap();
    let plain = dir.path().join("plain");
    let restored = dir.path().join("restored");
    let out = bin().arg("-o").arg(&plain).arg(&input).output().unwrap();
    assert!(out.status.success());
    let out = bin()
        .args(["--restore", "solid", "-o"])
        .arg(&restored)
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(code(&plain), code(&restored));
    assert!(!plain.join("Counter.jsx").exists());
    let component = std::fs::read_to_string(restored.join("Counter.jsx")).unwrap();
    assert!(
        component.contains("const [count, setCount] = createSignal(0);"),
        "{component}"
    );
    assert!(
        component.contains("<button onClick={() => setCount(count() + 1)}>+1</button>"),
        "{component}"
    );
}

#[test]
fn restore_svelte_and_solid_together_leave_the_code_alone() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bundle.js");
    std::fs::write(&input, fixture("bundle.iife.js")).unwrap();
    let plain = dir.path().join("plain");
    let restored = dir.path().join("restored");
    assert!(bin()
        .arg("-o")
        .arg(&plain)
        .arg(&input)
        .output()
        .unwrap()
        .status
        .success());
    let out = bin()
        .args(["--restore", "svelte,solid", "-o"])
        .arg(&restored)
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(code(&plain), code(&restored));
    for name in [
        "Parent.jsx",
        "Badge.jsx",
        "Label.jsx",
        "Context.jsx",
        "shared.jsx",
    ] {
        assert!(restored.join(name).exists(), "{name}");
    }
}

#[test]
fn a_solid_component_that_cannot_be_mapped_is_kept_with_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Odd.js");
    let code = fixture("Counter.js").replace("return _el$;", "_el$.foo();\n    return _el$;");
    std::fs::write(&input, code).unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .args(["--restore", "solid", "-v", "-o"])
        .arg(&outdir)
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!outdir.join("Counter.jsx").exists());
    assert!(String::from_utf8_lossy(&out.stderr).contains("solid: Counter kept as JS:"));
}
