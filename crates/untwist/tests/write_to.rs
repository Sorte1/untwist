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
        #[cfg(feature = "module-spans")]
        span: None,
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
            #[cfg(feature = "module-spans")]
            span: None,
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
        planned.iter().all(|file| file.contents != "module code"),
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
    let mut module_paths: Vec<String> = first_report
        .files
        .iter()
        .filter(|name| {
            name.as_str() != untwist::DEOBFUSCATED && name.as_str() != untwist::BUNDLE_JSON
        })
        .cloned()
        .collect();
    module_paths.sort();
    assert!(
        !module_paths.is_empty(),
        "the first run wrote no module files"
    );
    let second = Pipeline::new(Options {
        single_file: true,
        ..Options::default()
    })
    .run(WEBPACK_4);
    let report = second.write_to(dir.path()).expect("write");
    assert_eq!(
        report.files,
        vec![
            untwist::DEOBFUSCATED.to_string(),
            untwist::BUNDLE_JSON.to_string()
        ]
    );
    assert_eq!(report.stale, module_paths);
    for path in &module_paths {
        assert!(
            dir.path().join(path).is_file(),
            "{path} was removed by a write that was not asked to force"
        );
    }
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
