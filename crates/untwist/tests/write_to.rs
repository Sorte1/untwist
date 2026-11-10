mod common;

use std::path::PathBuf;

use common::WEBPACK_4;
use untwist::{BundleOutput, ModuleOutput, Options, Pipeline, PlanRefusal};

#[test]
fn write_to_writes_the_deobfuscated_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options::default()).run("var a=!0;");
    let report = out.write_to(dir.path()).expect("write");
    assert_eq!(report.files, vec![untwist::DEOBFUSCATED.to_string()]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(untwist::DEOBFUSCATED))
            .unwrap()
            .trim_end(),
        "var a = true;"
    );
}

#[test]
fn write_to_writes_the_bundle_and_its_modules() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let report = out.write_to(dir.path()).expect("write");
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(
        report.files,
        vec![
            untwist::DEOBFUSCATED.to_string(),
            untwist::BUNDLE_JSON.to_string(),
            "index.js".to_string(),
            "1.js".to_string()
        ]
    );
    assert!(dir.path().join(untwist::BUNDLE_JSON).is_file());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("1.js"))
            .unwrap()
            .trim_end(),
        "module.exports = 1;"
    );
}

#[test]
fn write_to_refuses_a_traversing_module_path() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("out");
    let mut out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let bundle = out.bundle.as_mut().expect("a bundle was detected");
    bundle.modules.push(ModuleOutput {
        id: "escape".to_string(),
        path: "../escape.js".to_string(),
        is_entry: false,
        code: "escaped();".to_string(),
        span: None,
        map: None,
    });
    let report = out.write_to(&dir).expect("write");
    assert!(
        report
            .refused
            .contains(&PlanRefusal::Unsafe("../escape.js".to_string())),
        "{:?}",
        report.refused
    );
    assert!(!report.files.iter().any(|file| file.contains("escape")));
    assert!(
        root.path().join("escape.js").symlink_metadata().is_err(),
        "the traversing module was written outside the output directory"
    );
    assert!(dir.join("index.js").is_file());
}

#[test]
fn several_inputs_get_distinct_deobfuscated_names() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options::default()).run_many(&["a();", "b();"]);
    let inputs = vec![PathBuf::from("a/bundle.js"), PathBuf::from("b/bundle.js")];
    let report = out
        .write_to_with(dir.path(), &inputs, false)
        .expect("write");
    assert_eq!(
        report.files,
        vec![
            "deobfuscated.bundle.js".to_string(),
            "deobfuscated.bundle-2.js".to_string()
        ]
    );
}

#[test]
fn single_file_output_concatenates_every_module_under_a_banner() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options {
        single_file: true,
        ..Options::default()
    })
    .run(WEBPACK_4);
    let bundle = out.bundle.as_ref().expect("a bundle was detected").clone();
    let report = out.write_to(dir.path()).expect("write");
    assert_eq!(
        report.files,
        vec![
            untwist::DEOBFUSCATED.to_string(),
            untwist::BUNDLE_JSON.to_string()
        ]
    );
    let text = std::fs::read_to_string(dir.path().join(untwist::DEOBFUSCATED)).unwrap();
    assert!(text.starts_with("// untwist: every module of this bundle, concatenated for reading"));
    let entry = bundle.entry().expect("an entry module").path.clone();
    let other = bundle
        .modules
        .iter()
        .find(|module| !module.is_entry)
        .expect("a second module")
        .path
        .clone();
    let entry_banner = format!("// ==== {entry} ====");
    let other_banner = format!("// ==== {other} ====");
    assert!(text.contains(&entry_banner), "{text}");
    assert!(text.contains(&other_banner), "{text}");
    assert!(
        text.find(&entry_banner) < text.find(&other_banner),
        "the entry must come first: {text}"
    );
    for module in &bundle.modules {
        assert!(
            dir.path().join(&module.path).symlink_metadata().is_err(),
            "{} was written as its own file",
            module.path
        );
    }
}

#[test]
fn single_file_output_without_a_bundle_is_just_the_code() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options {
        single_file: true,
        ..Options::default()
    })
    .run("var a=!0;");
    let report = out.write_to(dir.path()).expect("write");
    assert_eq!(report.files, vec![untwist::DEOBFUSCATED.to_string()]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(untwist::DEOBFUSCATED))
            .unwrap()
            .trim_end(),
        "var a = true;"
    );
}

#[test]
fn concatenated_banners_several_inputs_without_a_bundle_and_leaves_one_bare() {
    let options = Options {
        single_file: true,
        ..Options::default()
    };
    let two = Pipeline::new(options.clone()).run_many(&["a();", "b();"]);
    assert_eq!(
        two.concatenated(),
        "// ==== input 1 ====\na();\n\n// ==== input 2 ====\nb();\n"
    );
    let one = Pipeline::new(options).run("a();");
    assert_eq!(one.concatenated(), "a();");
}

#[test]
fn force_single_file_write_cleans_up_a_prior_per_module_run() {
    let dir = tempfile::tempdir().unwrap();
    let first = Pipeline::new(Options::default()).run(WEBPACK_4);
    let first_report = first.write_to(dir.path()).expect("write");
    let module_paths: Vec<String> = first_report
        .files
        .iter()
        .filter(|name| {
            name.as_str() != untwist::DEOBFUSCATED && name.as_str() != untwist::BUNDLE_JSON
        })
        .cloned()
        .collect();
    assert!(
        !module_paths.is_empty(),
        "the first run did not write any per-module files"
    );

    let second = Pipeline::new(Options {
        single_file: true,
        ..Options::default()
    })
    .run(WEBPACK_4);
    let second_report = second.write_to_with(dir.path(), &[], true).expect("write");
    assert_eq!(
        second_report.files,
        vec![
            untwist::DEOBFUSCATED.to_string(),
            untwist::BUNDLE_JSON.to_string()
        ]
    );
    assert!(
        second_report.refused.is_empty(),
        "{:?}",
        second_report.refused
    );
    for path in &module_paths {
        assert!(
            dir.path().join(path).symlink_metadata().is_err(),
            "{path} was not cleaned up by the single-file force write"
        );
    }
}

#[test]
fn a_module_colliding_with_the_deobfuscated_output_is_refused_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let mut out = Pipeline::new(Options::default()).run("var a=!0;");
    out.bundle = Some(BundleOutput {
        kind: "webpack".to_string(),
        entry_id: None,
        chunk_ids: Vec::new(),
        modules: vec![ModuleOutput {
            id: "./deobfuscated".to_string(),
            path: untwist::DEOBFUSCATED.to_string(),
            is_entry: false,
            code: "module code".to_string(),
            span: None,
            map: None,
        }],
    });
    let (planned, refused) = out.plan_write(dir.path(), &[]);
    assert!(!refused.is_empty(), "the collision was not reported");
    assert!(
        refused.iter().all(|refusal| matches!(
            refusal,
            PlanRefusal::Collision { path, .. } if path == untwist::DEOBFUSCATED
        )),
        "{refused:?}"
    );
    assert!(
        planned.iter().all(|file| file.contents != b"module code"),
        "the module overwrote the deobfuscated output"
    );
}

#[test]
fn a_write_into_a_fresh_directory_leaves_nothing_stale() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let report = out.write_to(dir.path()).expect("write");
    assert!(report.stale.is_empty(), "{:?}", report.stale);
}

#[test]
fn a_non_force_write_lists_the_module_files_a_previous_run_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let first = Pipeline::new(Options::default()).run(WEBPACK_4);
    let first_report = first.write_to(dir.path()).expect("write");
    let module_paths: Vec<String> = first_report
        .files
        .iter()
        .filter(|name| {
            name.as_str() != untwist::DEOBFUSCATED && name.as_str() != untwist::BUNDLE_JSON
        })
        .cloned()
        .collect();
    assert!(
        !module_paths.is_empty(),
        "the first run wrote no module files"
    );
    std::fs::remove_file(dir.path().join(untwist::DEOBFUSCATED)).unwrap();
    let second = Pipeline::new(Options::default()).run("var a=!0;");

    let report = second.write_to(dir.path()).expect("write");

    assert_eq!(report.files, vec![untwist::DEOBFUSCATED.to_string()]);
    for path in &module_paths {
        assert!(report.stale.contains(path), "{:?}", report.stale);
        assert!(
            dir.path().join(path).is_file(),
            "{path} was removed by a write that was not asked to force"
        );
    }
}

fn conflict_of(error: &anyhow::Error) -> &untwist::OutputConflict {
    error
        .downcast_ref::<untwist::OutputConflict>()
        .unwrap_or_else(|| panic!("not an output conflict: {error:#}"))
}

#[test]
fn a_second_write_without_force_is_a_conflict_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = Pipeline::new(Options::default()).run("var a=!0;");
    out.write_to(dir.path()).expect("write");
    let target = dir.path().join(untwist::DEOBFUSCATED);
    std::fs::write(&target, "edited by hand").unwrap();

    let error = out.write_to(dir.path()).expect_err("the file exists");

    assert_eq!(
        conflict_of(&error).paths,
        vec![untwist::DEOBFUSCATED.to_string()]
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "edited by hand");
}

#[test]
fn an_existing_file_at_a_module_path_is_a_conflict_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("1.js"), "keep me").unwrap();
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);

    let error = out.write_to(dir.path()).expect_err("the file exists");

    assert_eq!(conflict_of(&error).paths, vec!["1.js".to_string()]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("1.js")).unwrap(),
        "keep me"
    );
    assert!(!dir.path().join(untwist::DEOBFUSCATED).exists());
    assert!(!dir.path().join(untwist::BUNDLE_JSON).exists());
    assert!(!dir.path().join("index.js").exists());
}

fn listed_paths(dir: &std::path::Path) -> Vec<String> {
    let text = std::fs::read_to_string(dir.join(untwist::BUNDLE_JSON)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| module["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_forced_write_never_replaces_an_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("1.js");
    std::fs::write(&input, "the input").unwrap();
    let out = Pipeline::new(Options::default()).run(WEBPACK_4);

    let report = out
        .write_to_with(dir.path(), std::slice::from_ref(&input), true)
        .expect("write");

    assert_eq!(std::fs::read_to_string(&input).unwrap(), "the input");
    assert_eq!(
        report.refused,
        vec![PlanRefusal::InputFile("1.js".to_string())]
    );
    assert!(!report.files.contains(&"1.js".to_string()));
    assert_eq!(listed_paths(dir.path()), vec!["index.js".to_string()]);
}

#[test]
fn a_forced_write_never_removes_an_input_file_an_earlier_listing_names() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, "the input").unwrap();
    std::fs::write(
        dir.path().join(untwist::BUNDLE_JSON),
        r#"{"type":"webpack","entryId":"0","modules":[{"id":"9","path":"in.js"}]}"#,
    )
    .unwrap();
    let out = Pipeline::new(Options::default()).run("var a=!0;");

    let report = out
        .write_to_with(dir.path(), std::slice::from_ref(&input), true)
        .expect("write");

    assert_eq!(std::fs::read_to_string(&input).unwrap(), "the input");
    assert_eq!(report.files, vec![untwist::DEOBFUSCATED.to_string()]);
}

#[test]
fn bundle_json_lists_only_the_modules_that_were_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let bundle = out.bundle.as_mut().expect("a bundle was detected");
    bundle.modules.insert(
        1,
        ModuleOutput {
            id: "escape".to_string(),
            path: "../escape.js".to_string(),
            is_entry: false,
            code: "escaped();".to_string(),
            span: None,
            map: None,
        },
    );

    let report = out.write_to(dir.path()).expect("write");

    assert_eq!(
        listed_paths(dir.path()),
        vec!["index.js".to_string(), "1.js".to_string()]
    );
    assert_eq!(report.unwritten_modules, vec![1]);
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_written_is_refused_and_the_rest_is_written() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(locked.join("probe"), "").is_ok() {
        return;
    }
    let mut out = Pipeline::new(Options::default()).run(WEBPACK_4);
    let bundle = out.bundle.as_mut().expect("a bundle was detected");
    bundle.modules.insert(
        1,
        ModuleOutput {
            id: "locked".to_string(),
            path: "locked/a.js".to_string(),
            is_entry: false,
            code: "locked();".to_string(),
            span: None,
            map: None,
        },
    );

    let report = out.write_to(dir.path()).expect("write");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        matches!(
            report.refused.as_slice(),
            [PlanRefusal::Failed { path, .. }] if path == "locked/a.js"
        ),
        "{:?}",
        report.refused
    );
    assert_eq!(
        report.files,
        vec![
            untwist::DEOBFUSCATED.to_string(),
            untwist::BUNDLE_JSON.to_string(),
            "index.js".to_string(),
            "1.js".to_string()
        ]
    );
    assert_eq!(
        listed_paths(dir.path()),
        vec!["index.js".to_string(), "1.js".to_string()]
    );
}

fn bundle_of(paths: &[&str]) -> BundleOutput {
    BundleOutput {
        kind: "webpack".to_string(),
        entry_id: None,
        chunk_ids: Vec::new(),
        modules: paths
            .iter()
            .enumerate()
            .map(|(index, path)| ModuleOutput {
                id: index.to_string(),
                path: (*path).to_string(),
                is_entry: false,
                code: format!("module({index});"),
                span: None,
                map: None,
            })
            .collect(),
    }
}

fn written_with_modules(dir: &std::path::Path, paths: &[&str]) -> untwist::WriteReport {
    let mut out = Pipeline::new(Options::default()).run("var a=!0;");
    out.bundle = Some(bundle_of(paths));
    out.write_to(dir).expect("write")
}

fn module_files(report: &untwist::WriteReport) -> Vec<&str> {
    report
        .files
        .iter()
        .map(String::as_str)
        .filter(|name| *name != untwist::DEOBFUSCATED && *name != untwist::BUNDLE_JSON)
        .collect()
}

fn collided_paths(report: &untwist::WriteReport) -> Vec<&str> {
    report
        .refused
        .iter()
        .filter_map(|refusal| match refusal {
            PlanRefusal::Collision { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_module_path_under_another_modules_file_is_refused_and_the_rest_is_written() {
    let dir = tempfile::tempdir().unwrap();

    let report = written_with_modules(dir.path(), &["a.js", "a.js/b.js", "c.js/d.js", "c.js"]);

    assert_eq!(collided_paths(&report), vec!["a.js/b.js", "c.js"]);
    assert_eq!(module_files(&report), vec!["a.js", "c.js/d.js"]);
    assert_eq!(
        listed_paths(dir.path()),
        vec!["a.js".to_string(), "c.js/d.js".to_string()]
    );
}

#[test]
fn module_paths_differing_only_in_case_are_not_both_written() {
    let dir = tempfile::tempdir().unwrap();

    let report = written_with_modules(dir.path(), &["lib/Util.js", "LIB/util.js", "z.js"]);

    assert_eq!(collided_paths(&report), vec!["LIB/util.js"]);
    assert_eq!(module_files(&report), vec!["lib/Util.js", "z.js"]);
}

#[test]
fn a_module_path_no_file_system_takes_is_refused_and_the_rest_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let too_long = format!("{}.js", "a".repeat(300));

    let report = written_with_modules(dir.path(), &["a\0b.js", &too_long, "z.js"]);

    assert_eq!(
        report.refused,
        vec![
            PlanRefusal::Unsafe("a\0b.js".to_string()),
            PlanRefusal::Unsafe(too_long.clone())
        ]
    );
    assert_eq!(module_files(&report), vec!["z.js"]);
    assert_eq!(listed_paths(dir.path()), vec!["z.js".to_string()]);
}

#[test]
fn a_module_at_the_path_of_the_bundle_listing_does_not_replace_it() {
    let dir = tempfile::tempdir().unwrap();

    let report = written_with_modules(dir.path(), &["Bundle.json", "z.js"]);

    assert_eq!(collided_paths(&report), vec!["Bundle.json"]);
    assert_eq!(listed_paths(dir.path()), vec!["z.js".to_string()]);
}

#[test]
fn the_conflict_message_names_the_first_file_and_counts_the_rest() {
    let conflict = |paths: &[&str]| untwist::OutputConflict {
        paths: paths.iter().map(|path| path.to_string()).collect(),
    };

    assert_eq!(
        conflict(&["a.js"]).to_string(),
        "\"a.js\" already exists in the output directory"
    );
    assert_eq!(
        conflict(&["a.js", "b.js"]).to_string(),
        "\"a.js\" and 1 more file already exist in the output directory"
    );
    assert_eq!(
        conflict(&["a.js", "b.js", "c.js"]).to_string(),
        "\"a.js\" and 2 more files already exist in the output directory"
    );
}

#[test]
fn the_unpacker_reserves_the_names_the_writer_uses_itself() {
    use untwist_unpack::path::{RESERVED_DIRECTORIES, RESERVED_FILES};
    assert!(RESERVED_FILES.contains(&untwist::DEOBFUSCATED));
    assert!(RESERVED_FILES.contains(&untwist::BUNDLE_JSON));
    assert!(RESERVED_DIRECTORIES.contains(&untwist::WASM_DIR));
}

#[test]
fn a_force_write_leaves_nothing_stale_because_it_removed_it() {
    let dir = tempfile::tempdir().unwrap();
    let first = Pipeline::new(Options::default()).run(WEBPACK_4);
    first.write_to(dir.path()).expect("write");
    let second = Pipeline::new(Options {
        single_file: true,
        ..Options::default()
    })
    .run(WEBPACK_4);
    let report = second.write_to_with(dir.path(), &[], true).expect("write");
    assert!(report.stale.is_empty(), "{:?}", report.stale);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
}
