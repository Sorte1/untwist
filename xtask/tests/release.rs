use std::fs;
use std::path::Path;

use xtask::release::{
    bump, check_version, civil_date, internal_pins, package_versions, release_changelog,
    set_internal_pins, set_lock_version, set_package_version, set_workspace_version,
    workspace_version, Version,
};

const ROOT_TOML: &str = r#"[workspace]
members = ["crates/*", "vendor/regress", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
regress = { package = "untwist-regress", version = "=0.12.0", path = "vendor/regress" }
"#;

const CRATE_TOML: &str = r#"[package]
name = "untwist-unpack"
version.workspace = true

[dependencies]
untwist-core = { path = "../untwist-core", version = "0.1.0" }
regress.workspace = true

[dev-dependencies]
untwist-testing = { path = "../untwist-testing" }
"#;

const PACKAGE_JSON: &str = r#"{
  "name": "untwist",
  "version": "0.1.0",
  "dependencies": {
    "dep": {
      "version": "0.1.0"
    }
  }
}
"#;

const LOCK_JSON: &str = r#"{
  "name": "untwist-web",
  "version": "0.1.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "untwist-web",
      "version": "0.1.0",
      "license": "MIT"
    },
    "node_modules/dep": {
      "version": "0.1.0"
    }
  }
}
"#;

const CHANGELOG: &str =
    "# Changelog\n\nIntro.\n\n## [0.1.0] - Unreleased\n\nFirst public release.\n";

const CARGO_LOCK: &str = r#"version = 4

[[package]]
name = "anyhow"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "untwist-core"
version = "0.1.0"

[[package]]
name = "untwist-unpack"
version = "0.1.0"
dependencies = [
 "untwist-core",
]
"#;

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

#[test]
fn parses_plain_semver_and_orders_numerically() {
    assert_eq!(v("1.2.3").to_string(), "1.2.3");
    assert!(v("0.10.0") > v("0.9.9"));
    assert!(v("1.0.0") > v("0.99.99"));
}

#[test]
fn refuses_non_semver_versions() {
    for bad in [
        "", "1", "1.2", "1.2.3.4", "v1.2.3", "01.2.3", "1.2.x", "1.2.3-", " 1.2.3",
    ] {
        assert!(Version::parse(bad).is_err(), "{bad:?} should be refused");
    }
}

#[test]
fn reads_and_sets_the_workspace_version_only() {
    assert_eq!(workspace_version(ROOT_TOML).unwrap(), v("0.1.0"));
    let out = set_workspace_version(ROOT_TOML, &v("0.2.0")).unwrap();
    assert_eq!(workspace_version(&out).unwrap(), v("0.2.0"));
    assert!(out.contains(r#"version = "=0.12.0""#));
    assert_eq!(out.replace("0.2.0", "0.1.0"), ROOT_TOML);
}

#[test]
fn fails_when_the_workspace_has_no_version() {
    assert!(workspace_version("[workspace]\nmembers = []\n").is_err());
}

#[test]
fn sets_only_internal_version_pins() {
    let out = set_internal_pins(CRATE_TOML, &v("0.2.0"));
    assert_eq!(internal_pins(&out), vec!["0.2.0".to_string()]);
    assert_eq!(out.replace("0.2.0", "0.1.0"), CRATE_TOML);
}

#[test]
fn sets_the_top_level_package_json_version() {
    let out = set_package_version(PACKAGE_JSON, &v("0.2.0")).unwrap();
    assert_eq!(package_versions(&out).unwrap(), vec!["0.2.0".to_string()]);
    assert!(out.contains("      \"version\": \"0.1.0\""));
}

#[test]
fn sets_both_root_versions_of_a_package_lock() {
    let out = set_lock_version(LOCK_JSON, &v("0.2.0")).unwrap();
    assert_eq!(
        package_versions(&out).unwrap(),
        vec!["0.2.0".to_string(), "0.2.0".to_string()]
    );
    assert!(out.contains("\"node_modules/dep\": {\n      \"version\": \"0.1.0\""));
}

#[test]
fn dates_the_unreleased_section_and_opens_a_new_one() {
    let out = release_changelog(CHANGELOG, &v("0.2.0"), "2026-11-03").unwrap();
    assert_eq!(
        out,
        "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n## [0.2.0] - 2026-11-03\n\nFirst public release.\n"
    );
}

#[test]
fn dates_a_bare_unreleased_heading() {
    let text =
        "# Changelog\n\n## [Unreleased]\n\n### Fixed\n\n- A bug.\n\n## [0.1.0] - 2026-01-01\n";
    let out = release_changelog(text, &v("0.1.1"), "2026-11-03").unwrap();
    assert_eq!(
        out,
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.1] - 2026-11-03\n\n### Fixed\n\n- A bug.\n\n## [0.1.0] - 2026-01-01\n"
    );
}

#[test]
fn refuses_a_changelog_without_an_unreleased_section() {
    assert!(release_changelog(
        "# Changelog\n\n## [0.1.0] - 2026-01-01\n",
        &v("0.2.0"),
        "2026-11-03"
    )
    .is_err());
}

#[test]
fn formats_days_since_the_epoch_as_a_date() {
    assert_eq!(civil_date(0), "1970-01-01");
    assert_eq!(civil_date(11_016), "2000-02-29");
    assert_eq!(civil_date(20_760), "2026-11-03");
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn fake_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "Cargo.toml", ROOT_TOML);
    write(root, "Cargo.lock", CARGO_LOCK);
    write(root, "crates/untwist-unpack/Cargo.toml", CRATE_TOML);
    write(
        root,
        "crates/untwist-core/Cargo.toml",
        "[package]\nname = \"untwist-core\"\nversion.workspace = true\n",
    );
    write(root, "crates/untwist-wasm/npm/package.json", PACKAGE_JSON);
    write(
        root,
        "web/package.json",
        &PACKAGE_JSON.replace("\"untwist\"", "\"untwist-web\""),
    );
    write(root, "web/package-lock.json", LOCK_JSON);
    write(root, "CHANGELOG.md", CHANGELOG);
    dir
}

fn bump_lock(root: &Path, to: &str) {
    let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let lock = lock
        .replace(
            "name = \"untwist-core\"\nversion = \"0.1.0\"",
            &format!("name = \"untwist-core\"\nversion = \"{to}\""),
        )
        .replace(
            "name = \"untwist-unpack\"\nversion = \"0.1.0\"",
            &format!("name = \"untwist-unpack\"\nversion = \"{to}\""),
        );
    fs::write(root.join("Cargo.lock"), lock).unwrap();
}

#[test]
fn check_version_accepts_a_consistent_repo() {
    let repo = fake_repo();
    check_version(repo.path(), "v0.1.0").unwrap();
}

#[test]
fn check_version_rejects_a_tag_that_does_not_match() {
    let repo = fake_repo();
    let err = check_version(repo.path(), "v0.2.0")
        .unwrap_err()
        .to_string();
    assert!(err.contains("Cargo.toml"), "{err}");
    assert!(check_version(repo.path(), "0.1.0").is_err());
}

#[test]
fn check_version_names_each_stale_file() {
    let repo = fake_repo();
    write(
        repo.path(),
        "web/package.json",
        &PACKAGE_JSON.replace("0.1.0", "0.0.9"),
    );
    let err = check_version(repo.path(), "v0.1.0")
        .unwrap_err()
        .to_string();
    assert!(err.contains("web/package.json"), "{err}");
    assert!(!err.contains("crates/untwist-wasm"), "{err}");
}

#[test]
fn bump_updates_every_place_and_check_version_then_passes() {
    let repo = fake_repo();
    let root = repo.path();
    bump(root, "0.2.0", "2026-11-03").unwrap();
    let err = check_version(root, "v0.2.0").unwrap_err().to_string();
    assert!(err.contains("Cargo.lock"), "{err}");
    bump_lock(root, "0.2.0");
    check_version(root, "v0.2.0").unwrap();
    let changelog = fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    assert!(changelog.contains("## [Unreleased]\n\n## [0.2.0] - 2026-11-03\n"));
}

#[test]
fn bump_refuses_a_version_that_does_not_increase() {
    let repo = fake_repo();
    for bad in ["0.1.0", "0.0.9", "nope"] {
        assert!(bump(repo.path(), bad, "2026-11-03").is_err(), "{bad}");
    }
    let root_toml = fs::read_to_string(repo.path().join("Cargo.toml")).unwrap();
    assert_eq!(root_toml, ROOT_TOML);
}
