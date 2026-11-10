mod common;

use common::{bin, stdin_output, WEBPACK_4};
use untwist::source_map;

const SOURCE: &str = "var a=1,b=2;if(a)b(),a();";
const BUILT: &str = "var a=1;\nconsole.log(a);";
const ORIGINAL_MAP: &str = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["let a: number = 1\n\nconsole.log(a)\n"],"names":[],"mappings":"AAAA;AAEA"}"#;
const URL_COMMENT: &str = "//# sourceMappingURL=";

fn run_into(dir: &std::path::Path, input: &std::path::Path, args: &[&str]) -> std::path::PathBuf {
    let out_dir = dir.join(format!("out{}", args.len()));
    let out = bin()
        .args(args)
        .arg("-o")
        .arg(&out_dir)
        .arg(input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out_dir
}

fn written_input(dir: &std::path::Path, name: &str, text: &str) -> std::path::PathBuf {
    let input = dir.join(name);
    std::fs::write(&input, text).unwrap();
    input
}

#[test]
fn a_file_map_is_written_beside_the_output_and_linked_from_it() {
    let dir = tempfile::tempdir().unwrap();
    let input = written_input(dir.path(), "in.js", SOURCE);
    let plain = run_into(dir.path(), &input, &[]);
    let mapped = run_into(dir.path(), &input, &["--source-map=file"]);
    let plain_code = std::fs::read_to_string(plain.join("deobfuscated.js")).unwrap();
    let mapped_code = std::fs::read_to_string(mapped.join("deobfuscated.js")).unwrap();
    let (code, link) = mapped_code
        .rsplit_once(URL_COMMENT)
        .expect("a sourceMappingURL comment");
    assert_eq!(code.trim_end(), plain_code.trim_end());
    assert_eq!(link, "deobfuscated.js.map");
    let json = std::fs::read_to_string(mapped.join("deobfuscated.js.map")).unwrap();
    let map = source_map::parse(&json).expect("a source map");
    assert_eq!(map.get_source(0), Some("in.js"));
    assert_eq!(map.get_file(), Some("deobfuscated.js"));
    assert!(map.get_tokens().count() > 0);
}

#[test]
fn an_inline_map_goes_to_stdout_as_a_data_url() {
    let plain = stdin_output(SOURCE, &[]);
    let mapped = stdin_output(SOURCE, &["--source-map=inline"]);
    assert!(mapped.status.success());
    let plain = String::from_utf8(plain.stdout).unwrap();
    let mapped = String::from_utf8(mapped.stdout).unwrap();
    let (code, url) = mapped
        .rsplit_once(URL_COMMENT)
        .expect("a sourceMappingURL comment");
    assert_eq!(code.trim_end(), plain.trim_end());
    assert!(url.starts_with("data:application/json"));
    let embedded = untwist::Output::input_source_map(&mapped).expect("an embedded map");
    let map = source_map::parse(&embedded).expect("a source map");
    assert_eq!(map.get_source(0), Some("stdin"));
}

#[test]
fn a_file_map_without_an_output_directory_is_refused() {
    let out = stdin_output(SOURCE, &["--source-map=file"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--output"));
}

#[test]
fn an_input_map_file_maps_through_to_the_original_source() {
    let dir = tempfile::tempdir().unwrap();
    let input = written_input(dir.path(), "built.js", BUILT);
    let given = written_input(dir.path(), "built.js.map", ORIGINAL_MAP);
    let given = given.to_str().unwrap();
    let out_dir = run_into(
        dir.path(),
        &input,
        &["--source-map=file", "--input-source-map", given],
    );
    let json = std::fs::read_to_string(out_dir.join("deobfuscated.js.map")).unwrap();
    let map = source_map::parse(&json).expect("a source map");
    assert_eq!(map.get_source(0), Some("src/app.ts"));
}

#[test]
fn an_input_map_that_is_not_a_map_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let input = written_input(dir.path(), "built.js", BUILT);
    let given = written_input(dir.path(), "junk.map", "not json");
    let out = bin()
        .args(["--source-map=inline", "--input-source-map"])
        .arg(&given)
        .arg(&input)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("is not a source map"));
}

#[test]
fn every_unpacked_module_file_gets_its_own_map() {
    let dir = tempfile::tempdir().unwrap();
    let input = written_input(dir.path(), "bundle.js", WEBPACK_4);
    let out_dir = run_into(dir.path(), &input, &["--source-map=file"]);
    let modules: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "js")
                && path
                    .file_name()
                    .is_some_and(|name| name != "deobfuscated.js")
        })
        .collect();
    assert!(!modules.is_empty());
    for module in modules {
        let code = std::fs::read_to_string(&module).unwrap();
        assert!(code.contains(URL_COMMENT), "{}", module.display());
        let json = std::fs::read_to_string(module.with_extension("js.map")).unwrap();
        let map = source_map::parse(&json).expect("a source map");
        assert_eq!(map.get_source(0), Some("bundle.js"));
    }
}
