mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::bin;

const ENTRY: (&str, &str) = ("./index", "console.log(1);");

fn bundle_source(modules: &[(&str, &str)]) -> String {
    let table: Vec<String> = modules
        .iter()
        .map(|(id, body)| {
            format!("\"{id}\":function(module,exports,__webpack_require__){{{body}}}")
        })
        .collect();
    format!(
        "(function(modules){{var cache={{}};function __webpack_require__(id){{\
         if(cache[id])return cache[id].exports;var module=cache[id]={{exports:{{}}}};\
         modules[id].call(module.exports,module,module.exports,__webpack_require__);\
         return module.exports;}}return __webpack_require__(\"./index\");}})({{{}}});",
        table.join(",")
    )
}

struct Run {
    _dir: tempfile::TempDir,
    input: PathBuf,
    outdir: PathBuf,
}

fn prepared(modules: &[(&str, &str)]) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, bundle_source(modules)).unwrap();
    let outdir = dir.path().join("out");
    Run {
        _dir: dir,
        input,
        outdir,
    }
}

fn unpack(run: &Run, flags: &[&str]) -> Output {
    bin()
        .arg("-o")
        .arg(&run.outdir)
        .args(flags)
        .arg(&run.input)
        .output()
        .unwrap()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn listed(outdir: &Path) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(outdir.join("bundle.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| {
            (
                module["id"].as_str().unwrap().to_string(),
                module["path"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn path_of(listing: &[(String, String)], id: &str) -> String {
    listing
        .iter()
        .find(|(listed_id, _)| listed_id == id)
        .unwrap_or_else(|| panic!("{id:?} is not listed in {listing:?}"))
        .1
        .clone()
}

fn files_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(
                    path.strip_prefix(dir)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    found.sort();
    found
}

fn assert_every_listed_module_is_a_file(outdir: &Path, expected: usize) {
    let listing = listed(outdir);
    assert_eq!(listing.len(), expected, "{listing:?}");
    for (id, path) in &listing {
        assert!(outdir.join(path).is_file(), "{id:?} at {path:?} is missing");
    }
    let folded: std::collections::HashSet<String> = listing
        .iter()
        .map(|(_, path)| path.to_lowercase())
        .collect();
    assert_eq!(folded.len(), listing.len(), "{listing:?}");
}

#[test]
fn hidden_and_tool_directory_names_get_generated_names() {
    let run = prepared(&[
        ENTRY,
        ("./.eslintrc", "module.exports=1;"),
        ("./.git/hooks/x", "module.exports=2;"),
        ("./package.json", "module.exports=3;"),
        ("./in", "module.exports=4;"),
    ]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!run.outdir.join(".eslintrc.js").exists());
    assert!(!run.outdir.join(".git").exists());
    assert!(run.outdir.join("in.js").is_file());
    assert!(run.outdir.join("package.json.js").is_file());
    let listing = listed(&run.outdir);
    assert!(path_of(&listing, "./.eslintrc").starts_with("_unsafe_"));
    assert!(path_of(&listing, "./.git/hooks/x").starts_with("_unsafe_"));
    assert_every_listed_module_is_a_file(&run.outdir, 5);
    assert!(
        files_under(&run.outdir)
            .iter()
            .all(|file| !file.split('/').any(|segment| segment.starts_with('.'))),
        "{:?}",
        files_under(&run.outdir)
    );
}

#[test]
fn the_current_directory_is_refused_as_the_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let source = bundle_source(&[ENTRY, ("./in", "module.exports=4;")]);
    std::fs::write(dir.path().join("in.js"), &source).unwrap();

    let out = bin()
        .current_dir(dir.path())
        .args(["in.js", "-o", "."])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert!(stderr_of(&out).contains("error: refusing to write into"));
    assert_eq!(files_under(dir.path()), vec!["in.js".to_string()]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("in.js")).unwrap(),
        source
    );
}

#[test]
fn an_existing_file_a_module_would_replace_is_a_conflict_without_force() {
    let run = prepared(&[ENTRY, ("./z", "module.exports=7;")]);
    std::fs::create_dir(&run.outdir).unwrap();
    std::fs::write(run.outdir.join("z.js"), "keep me").unwrap();

    let out = unpack(&run, &[]);

    assert_eq!(out.status.code(), Some(2), "{}", stderr_of(&out));
    assert!(stderr_of(&out).contains("z.js"), "{}", stderr_of(&out));
    assert!(stderr_of(&out).contains("--force"), "{}", stderr_of(&out));
    assert_eq!(files_under(&run.outdir), vec!["z.js".to_string()]);
    assert_eq!(
        std::fs::read_to_string(run.outdir.join("z.js")).unwrap(),
        "keep me"
    );
}

#[test]
fn an_existing_file_where_a_module_directory_would_go_is_a_conflict_without_force() {
    let run = prepared(&[ENTRY, ("./sub/z", "module.exports=7;")]);
    std::fs::create_dir(&run.outdir).unwrap();
    std::fs::write(run.outdir.join("sub"), "keep me").unwrap();

    let out = unpack(&run, &[]);

    assert_eq!(out.status.code(), Some(2), "{}", stderr_of(&out));
    assert_eq!(files_under(&run.outdir), vec!["sub".to_string()]);
}

#[test]
fn an_existing_directory_with_unrelated_files_is_written_into_without_force() {
    let run = prepared(&[ENTRY, ("./sub/z", "module.exports=7;")]);
    std::fs::create_dir_all(run.outdir.join("sub")).unwrap();
    std::fs::write(run.outdir.join("notes.txt"), "keep me").unwrap();
    std::fs::write(run.outdir.join("sub/other.txt"), "keep me too").unwrap();

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(run.outdir.join("sub/z.js").is_file());
    assert_eq!(
        std::fs::read_to_string(run.outdir.join("notes.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(
        std::fs::read_to_string(run.outdir.join("sub/other.txt")).unwrap(),
        "keep me too"
    );
}

#[test]
fn force_never_overwrites_an_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let outdir = dir.path().join("work");
    std::fs::create_dir(&outdir).unwrap();
    let input = outdir.join("in.js");
    let source = bundle_source(&[
        ENTRY,
        ("./in", "module.exports=4;"),
        ("./z", "module.exports=7;"),
    ]);
    std::fs::write(&input, &source).unwrap();

    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_eq!(std::fs::read_to_string(&input).unwrap(), source);
    assert!(
        stderr_of(&out).contains("it is one of the input files"),
        "{}",
        stderr_of(&out)
    );
    assert!(outdir.join("z.js").is_file());
    let listing = listed(&outdir);
    assert!(
        listing.iter().all(|(id, _)| id != "./in"),
        "an unwritten module is listed: {listing:?}"
    );
}

#[test]
fn force_never_deletes_an_input_file_an_earlier_listing_names() {
    let dir = tempfile::tempdir().unwrap();
    let outdir = dir.path().join("work");
    std::fs::create_dir(&outdir).unwrap();
    let input = outdir.join("in.js");
    std::fs::write(&input, "x()").unwrap();
    std::fs::write(
        outdir.join("bundle.json"),
        r#"{"type":"webpack","entryId":"0","modules":[{"id":"9","path":"in.js"}]}"#,
    )
    .unwrap();

    let out = bin()
        .arg("-o")
        .arg(&outdir)
        .arg("-f")
        .arg(&input)
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_eq!(std::fs::read_to_string(&input).unwrap(), "x()");
    assert!(outdir.join("deobfuscated.js").is_file());
}

#[test]
fn a_module_id_under_another_modules_file_does_not_stop_the_write() {
    let run = prepared(&[
        ENTRY,
        ("./a\\u001b[31mRED", "module.exports=4;"),
        ("./a.js", "module.exports=5;"),
        ("./a.js/b", "module.exports=6;"),
        ("./z", "module.exports=7;"),
    ]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(run.outdir.join("z.js").is_file());
    assert!(run.outdir.join("a.js").is_file());
    assert_every_listed_module_is_a_file(&run.outdir, 5);
    assert!(
        files_under(&run.outdir)
            .iter()
            .all(|file| !file.chars().any(char::is_control)),
        "{:?}",
        files_under(&run.outdir)
    );
}

#[test]
fn ids_differing_only_in_case_and_device_names_get_portable_names() {
    let run = prepared(&[
        ENTRY,
        ("./A", "module.exports=5;"),
        ("./a", "module.exports=6;"),
        ("./CON", "module.exports=7;"),
    ]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_every_listed_module_is_a_file(&run.outdir, 4);
    let listing = listed(&run.outdir);
    assert_eq!(path_of(&listing, "./A"), "A.js");
    assert_eq!(path_of(&listing, "./a"), "a-2.js");
    assert!(path_of(&listing, "./CON").starts_with("_unsafe_"));
}

#[test]
fn a_module_id_longer_than_a_file_name_does_not_stop_the_write() {
    let long_id = format!("./{}", "a".repeat(300));
    let run = prepared(&[
        ENTRY,
        (&long_id, "module.exports=4;"),
        ("./z", "module.exports=7;"),
    ]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(run.outdir.join("z.js").is_file());
    assert_every_listed_module_is_a_file(&run.outdir, 3);
    assert!(path_of(&listed(&run.outdir), &long_id).len() <= 255);
}

#[test]
fn a_module_id_holding_a_nul_does_not_stop_the_write() {
    let run = prepared(&[
        ENTRY,
        ("./a\\u0000b", "module.exports=4;"),
        ("./z", "module.exports=7;"),
    ]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(run.outdir.join("z.js").is_file());
    assert_every_listed_module_is_a_file(&run.outdir, 3);
    assert!(path_of(&listed(&run.outdir), "./a\u{0}b").starts_with("_unsafe_"));
}

#[test]
fn a_module_named_like_the_tools_own_files_keeps_them_intact() {
    let run = prepared(&[ENTRY, ("./deobfuscated", "module.exports=4;")]);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains("refusing"), "{}", stderr_of(&out));
    let listing = listed(&run.outdir);
    assert_eq!(path_of(&listing, "./deobfuscated"), "deobfuscated-2.js");
    assert_every_listed_module_is_a_file(&run.outdir, 2);
    let whole = std::fs::read_to_string(run.outdir.join("deobfuscated.js")).unwrap();
    let module = std::fs::read_to_string(run.outdir.join("deobfuscated-2.js")).unwrap();
    assert!(whole.contains("console.log(1)"), "{whole}");
    assert!(!module.contains("console.log(1)"), "{module}");
}

const MANY_COLLIDING_IDS: usize = 2500;

#[test]
fn thousands_of_ids_that_normalise_to_one_path_are_all_written() {
    let ids: Vec<String> = (0..MANY_COLLIDING_IDS)
        .map(|index| format!("./x{index}/../a"))
        .collect();
    let mut modules = vec![ENTRY];
    modules.extend(ids.iter().map(|id| (id.as_str(), "module.exports=1;")));
    let run = prepared(&modules);

    let out = unpack(&run, &[]);

    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains("refusing"), "{}", stderr_of(&out));
    assert_every_listed_module_is_a_file(&run.outdir, MANY_COLLIDING_IDS + 1);
    assert!(run
        .outdir
        .join(format!("a-{MANY_COLLIDING_IDS}.js"))
        .is_file());
}

#[cfg(unix)]
fn made_read_only(dir: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let probe = dir.join("probe");
    let writable = std::fs::write(&probe, "").is_ok();
    if writable {
        std::fs::remove_file(&probe).unwrap();
    }
    !writable
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_written_is_reported_and_the_rest_is_written() {
    use std::os::unix::fs::PermissionsExt;
    let run = prepared(&[
        ENTRY,
        ("./locked/a", "module.exports=4;"),
        ("./z", "module.exports=7;"),
    ]);
    let locked = run.outdir.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    if !made_read_only(&locked) {
        return;
    }

    let out = unpack(&run, &[]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert!(
        stderr_of(&out).contains("warning: could not write \"locked/a.js\""),
        "{}",
        stderr_of(&out)
    );
    assert!(run.outdir.join("z.js").is_file());
    assert!(run.outdir.join("index.js").is_file());
    let listing = listed(&run.outdir);
    assert!(
        listing.iter().all(|(id, _)| id != "./locked/a"),
        "an unwritten module is listed: {listing:?}"
    );
    assert_every_listed_module_is_a_file(&run.outdir, 2);
}

#[cfg(unix)]
#[test]
fn the_json_report_lists_only_the_modules_that_were_written() {
    use std::os::unix::fs::PermissionsExt;
    let run = prepared(&[
        ENTRY,
        ("./locked/a", "module.exports=4;"),
        ("./z", "module.exports=7;"),
    ]);
    let locked = run.outdir.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    if !made_read_only(&locked) {
        return;
    }

    let out = unpack(&run, &["--report", "json"]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ids: Vec<&str> = report["bundle"]["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| module["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["./index", "./z"]);
    let files: Vec<&str> = report["files_written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        files,
        vec!["deobfuscated.js", "bundle.json", "index.js", "z.js"]
    );
}
