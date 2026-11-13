use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::{unpack_named_inputs, Bundle, UnpackOptions};

const BUNDLE: &str = r#"(function (modules) {
  var cache = {};
  function __webpack_require__(id) {
    if (cache[id]) return cache[id].exports;
    var module = cache[id] = { exports: {} };
    modules[id].call(module.exports, module, module.exports, __webpack_require__);
    return module.exports;
  }
  return __webpack_require__("./index");
})({
  "./index": function (module, exports, __webpack_require__) { console.log(__webpack_require__("./lazy")); },
  "./lazy": function (module) { module.exports = "lazy module"; }
});"#;

const PLAIN: &str = "console.log(\"a plain script\");";

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_string()).collect()
}

fn paths(bundle: &Bundle<'_>) -> Vec<(String, String, bool)> {
    bundle
        .modules
        .values()
        .map(|module| {
            (
                module.id.clone(),
                module.path.as_str().to_string(),
                module.is_entry,
            )
        })
        .collect()
}

fn unpacked(sources: &[&str], given: &[String]) -> (Vec<(String, String, bool)>, Vec<String>) {
    let alloc = Allocator::default();
    let mut programs: Vec<_> = sources
        .iter()
        .map(|source| parse_program(&alloc, source).program)
        .collect();
    let (bundle, _) =
        unpack_named_inputs(&alloc, &mut programs, &[], UnpackOptions::default(), given);
    let bundle = bundle.expect("the bundle is unpacked");
    (paths(&bundle), bundle.mirrored_inputs.clone())
}

#[test]
fn a_bundle_beside_a_plain_input_keeps_every_input_name_free() {
    let given = names(&["index.js", "lazy.js"]);

    let (paths, mirrored) = unpacked(&[BUNDLE, PLAIN], &given);

    assert_eq!(mirrored, given);
    for (id, path, _) in &paths {
        assert!(!given.contains(path), "{id} took the input name {path}");
    }
    assert!(paths
        .iter()
        .any(|(id, _, is_entry)| id == "./index" && *is_entry));
}

#[test]
fn an_entry_module_beside_an_input_named_index_js_takes_its_id_path() {
    let given = names(&["index.js", "main.js"]);

    let (paths, _) = unpacked(&[PLAIN, BUNDLE], &given);

    let entry = paths.iter().find(|(_, _, is_entry)| *is_entry).unwrap();
    assert_ne!(entry.1, "index.js");
    assert!(!given.contains(&entry.1), "{paths:?}");
}

#[test]
fn a_bundle_that_takes_every_input_mirrors_nothing() {
    let (paths, mirrored) = unpacked(&[BUNDLE], &names(&["index.js"]));

    assert!(mirrored.is_empty());
    assert!(paths
        .iter()
        .any(|(_, path, is_entry)| path == "index.js" && *is_entry));
}

#[test]
fn without_input_names_a_partial_bundle_keeps_its_paths() {
    let (paths, mirrored) = unpacked(&[BUNDLE, PLAIN], &[]);

    assert!(mirrored.is_empty());
    assert!(paths
        .iter()
        .any(|(_, path, is_entry)| path == "index.js" && *is_entry));
    assert!(paths.iter().any(|(_, path, _)| path == "lazy.js"));
}

#[test]
fn input_names_that_do_not_match_the_inputs_are_ignored() {
    let (paths, mirrored) = unpacked(&[BUNDLE, PLAIN], &names(&["only-one.js"]));

    assert!(mirrored.is_empty());
    assert!(paths.iter().any(|(_, path, _)| path == "index.js"));
}

#[test]
fn an_input_in_a_directory_reserves_that_directory_for_files() {
    let given = names(&["index.js", "lazy.js/inner.js"]);

    let (paths, _) = unpacked(&[BUNDLE, PLAIN], &given);

    assert!(
        paths.iter().all(|(_, path, _)| path != "lazy.js"),
        "{paths:?}"
    );
}
