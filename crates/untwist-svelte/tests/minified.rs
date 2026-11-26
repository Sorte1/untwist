use std::path::Path;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).expect("fixture reads")
}

fn hashes(code: &str) -> Vec<(String, u64)> {
    untwist_svelte::fingerprint_rows(code)
        .into_iter()
        .map(|row| (row.name, row.hash))
        .collect()
}

#[test]
fn a_function_shape_does_not_depend_on_its_names() {
    let left =
        hashes("function a(x, y) { const z = x.f; return z + b(y); } function b(q) { return q; }");
    let right =
        hashes("function k(m, n) { const p = m.f; return p + w(n); } function w(r) { return r; }");
    assert_eq!(left.len(), 2);
    assert_eq!(left[0].1, right[0].1);
    assert_eq!(left[1].1, right[1].1);
}

#[test]
fn a_function_shape_keeps_property_names_and_literals() {
    let base = hashes("function a(x) { return x.f + 1; }");
    let property = hashes("function a(x) { return x.g + 1; }");
    let literal = hashes("function a(x) { return x.f + 2; }");
    let global = hashes("function a(x) { return document.f + 1; }");
    assert_ne!(base[0].1, property[0].1);
    assert_ne!(base[0].1, literal[0].1);
    assert_ne!(base[0].1, global[0].1);
}

#[test]
fn a_function_shape_tells_inner_bindings_from_outer_ones() {
    let inner = hashes("function a(x) { const y = 1; return x + y; } const y = 2;");
    let outer = hashes("function a(x) { return x + y; } const y = 2;");
    assert_ne!(inner[0].1, outer[0].1);
}

#[test]
fn minified_bundles_restore_their_components() {
    for (name, least) in [
        ("bundle.terser.untwisted.js", 4),
        ("bundle.vite.untwisted.js", 7),
    ] {
        let restore = untwist_svelte::restore(&fixture(name));
        assert!(
            restore.components.len() >= least,
            "{name}: {} restored, kept {:?}",
            restore.components.len(),
            restore.kept
        );
    }
}

#[test]
fn a_runtime_shape_found_twice_is_not_trusted() {
    for name in ["bundle.terser.untwisted.js", "bundle.vite.untwisted.js"] {
        let code = fixture(name);
        let copy = match code.trim_start().starts_with("(function") {
            true => code.clone(),
            false => format!("(function () {{\n{code}\n}})();"),
        };
        assert!(!untwist_svelte::restore(&code).components.is_empty());
        let doubled = format!("{code}\n{copy}\n");
        let restore = untwist_svelte::restore(&doubled);
        assert!(
            restore.components.is_empty(),
            "{name}: a doubled runtime still restored {}",
            restore.components.len()
        );
    }
}
