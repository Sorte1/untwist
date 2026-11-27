mod common;

use common::bin;

#[test]
fn restore_solid_reads_minified_terser_and_vite_bundles_without_changing_the_code() {
    let raw = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist-solid/tests/fixtures/raw");
    for (name, least) in [("bundle.vite.js", 7), ("bundle.terser.js", 2)] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join(name);
        std::fs::copy(raw.join(name), &input).unwrap();
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
        let code =
            |dir: &std::path::Path| std::fs::read_to_string(dir.join("deobfuscated.js")).unwrap();
        assert!(!code(&restored).contains("_2__"), "{}", code(&plain));
        let written = std::fs::read_dir(&restored)
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "jsx"))
            })
            .count();
        assert!(written >= least, "{name}: {written} components");
    }
}
