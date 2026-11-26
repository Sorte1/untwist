mod common;

use common::bin;

const COUNTER: &str = "../untwist-svelte/tests/fixtures/Counter.js";

fn counter() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(COUNTER);
    std::fs::read_to_string(path).expect("the Counter fixture reads")
}

#[test]
fn restore_svelte_writes_the_component_beside_the_unchanged_code() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Counter.js");
    std::fs::write(&input, counter()).unwrap();
    let plain = dir.path().join("plain");
    let restored = dir.path().join("restored");
    let out = bin().arg("-o").arg(&plain).arg(&input).output().unwrap();
    assert!(out.status.success());
    let out = bin()
        .args(["--restore", "svelte", "-o"])
        .arg(&restored)
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let code =
        |dir: &std::path::Path| std::fs::read_to_string(dir.join("deobfuscated.js")).unwrap();
    assert_eq!(code(&plain), code(&restored));
    assert!(!plain.join("Counter.svelte").exists());
    let component = std::fs::read_to_string(restored.join("Counter.svelte")).unwrap();
    assert!(component.contains("let count = $state(0);"), "{component}");
    assert!(
        component.contains("<button onclick={increment}>clicks: {count}</button>"),
        "{component}"
    );
}

#[test]
fn restore_svelte_needs_an_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Counter.js");
    std::fs::write(&input, counter()).unwrap();
    let out = bin()
        .args(["--restore", "svelte"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--restore needs --output DIR"));
}

#[test]
fn a_component_that_cannot_be_mapped_is_kept_with_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Odd.js");
    let code = counter().replace(
        "$.append($$anchor, fragment);",
        "$.append($$anchor, fragment);\n\t$.untrack(() => 1);",
    );
    std::fs::write(&input, code).unwrap();
    let outdir = dir.path().join("out");
    let out = bin()
        .args(["--restore", "svelte", "-v", "-o"])
        .arg(&outdir)
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!outdir.join("Counter.svelte").exists());
    assert!(String::from_utf8_lossy(&out.stderr).contains("svelte: Counter kept as JS:"));
}

#[test]
fn restore_svelte_reads_minified_terser_and_vite_bundles_without_changing_the_code() {
    for (name, least) in [("bundle.terser.js", 6), ("bundle.vite.js", 7)] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../untwist-svelte/tests/fixtures/raw")
            .join(name);
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain");
        let restored = dir.path().join("restored");
        let out = bin().arg("-o").arg(&plain).arg(&path).output().unwrap();
        assert!(out.status.success());
        let out = bin()
            .args(["--restore", "svelte", "-o"])
            .arg(&restored)
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let code =
            |dir: &std::path::Path| std::fs::read_to_string(dir.join("deobfuscated.js")).unwrap();
        assert_eq!(code(&plain), code(&restored));
        let components = std::fs::read_dir(&restored)
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "svelte"))
            })
            .count();
        assert!(
            components >= least,
            "{name}: {components} components restored"
        );
    }
}
