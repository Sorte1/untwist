use untwist_unpack::specifier::ModulePaths;

fn assigned(paths: &[&str]) -> ModulePaths {
    ModulePaths::from_pairs(paths.iter().map(|path| (*path, *path)))
}

#[test]
fn the_reference_form_is_bare_for_anything_under_the_root_node_modules() {
    let paths = ModulePaths::default().matching_reference(true);
    assert_eq!(paths.specifier("a.js", "x/y.js"), "./x/y.js");
    assert_eq!(paths.specifier("x/y.js", "a.js"), "../a.js");
    assert_eq!(paths.specifier("a.js", "node_modules/lib"), "lib");
    assert_eq!(paths.specifier("utils/color.js", "1.js"), "../1.js");
    assert_eq!(paths.specifier("index.js", "1.js"), "./1.js");
}

#[test]
fn a_path_outside_node_modules_is_relative() {
    let paths = assigned(&["a.js", "x/y.js", "utils/color.js", "1.js"]);
    assert_eq!(paths.specifier("a.js", "x/y.js"), "./x/y.js");
    assert_eq!(paths.specifier("x/y.js", "a.js"), "../a.js");
    assert_eq!(paths.specifier("utils/color.js", "1.js"), "../1.js");
    assert_eq!(paths.specifier("a.js", "./1.js"), "./1.js");
}

#[test]
fn a_dot_directory_beside_the_requester_is_not_read_as_a_package() {
    let paths = assigned(&["index.js", ".config/a.js"]);
    assert_eq!(
        paths.specifier("index.js", ".config/a.js"),
        "./.config/a.js"
    );
}

#[test]
fn an_assigned_node_modules_path_nothing_nearer_matches_is_bare() {
    let paths = assigned(&[
        "index.js",
        "a/main.js",
        "node_modules/y.js",
        "node_modules/lib/index.js",
        "b/node_modules/y.js",
    ]);
    assert_eq!(paths.specifier("index.js", "node_modules/y.js"), "y.js");
    assert_eq!(paths.specifier("a/main.js", "node_modules/y.js"), "y.js");
    assert_eq!(
        paths.specifier("a/main.js", "node_modules/lib/index.js"),
        "lib/index.js"
    );
    assert_eq!(
        paths.specifier("b/node_modules/y.js", "a/main.js"),
        "../../a/main.js"
    );
}

#[test]
fn a_nearer_node_modules_file_makes_the_specifier_relative() {
    let paths = assigned(&[
        "a/b/main.js",
        "node_modules/y.js",
        "a/node_modules/y.js",
        "node_modules/lib-x/main",
        "a/b/node_modules/lib-x/main.json",
    ]);
    assert_eq!(
        paths.specifier("a/b/main.js", "node_modules/y.js"),
        "../../node_modules/y.js"
    );
    assert_eq!(
        paths.specifier("a/b/main.js", "node_modules/lib-x/main"),
        "../../node_modules/lib-x/main"
    );
    assert_eq!(paths.specifier("c/main.js", "node_modules/y.js"), "y.js");
}

#[test]
fn a_nearer_node_modules_directory_makes_the_specifier_relative() {
    let paths = assigned(&[
        "a/main.js",
        "node_modules/y.js",
        "a/node_modules/y.js/index.js",
    ]);
    assert_eq!(
        paths.specifier("a/main.js", "node_modules/y.js"),
        "../node_modules/y.js"
    );
}

#[test]
fn a_package_inside_node_modules_sees_its_own_nested_copy_first() {
    let paths = assigned(&[
        "node_modules/a/index.js",
        "node_modules/b/index.js",
        "node_modules/c/index.js",
        "node_modules/a/node_modules/b/index.js",
    ]);
    assert_eq!(
        paths.specifier("node_modules/a/index.js", "node_modules/b/index.js"),
        "../b/index.js"
    );
    assert_eq!(
        paths.specifier("node_modules/a/index.js", "node_modules/c/index.js"),
        "c/index.js"
    );
}

#[test]
fn a_name_node_reads_as_a_core_module_is_relative() {
    let paths = assigned(&[
        "index.js",
        "node_modules/events",
        "node_modules/fs/promises",
        "node_modules/node:fs",
        "node_modules/#internal/a.js",
        "node_modules/events/index.js",
        "node_modules/events.js",
    ]);
    assert_eq!(
        paths.specifier("index.js", "node_modules/events"),
        "./node_modules/events"
    );
    assert_eq!(
        paths.specifier("index.js", "node_modules/fs/promises"),
        "./node_modules/fs/promises"
    );
    assert_eq!(
        paths.specifier("index.js", "node_modules/node:fs"),
        "./node_modules/node:fs"
    );
    assert_eq!(
        paths.specifier("index.js", "node_modules/#internal/a.js"),
        "./node_modules/#internal/a.js"
    );
    assert_eq!(
        paths.specifier("index.js", "node_modules/events/index.js"),
        "events/index.js"
    );
    assert_eq!(
        paths.specifier("index.js", "node_modules/events.js"),
        "events.js"
    );
}

#[test]
fn a_path_that_was_not_assigned_is_relative() {
    let paths = assigned(&["index.js"]);
    assert_eq!(
        paths.specifier("index.js", "node_modules/y.js"),
        "./node_modules/y.js"
    );
    assert_eq!(
        ModulePaths::default().specifier("a/b.js", "runtime.js"),
        "../runtime.js"
    );
}

#[test]
fn an_id_maps_to_its_assigned_path() {
    let paths = ModulePaths::from_pairs([("7", "./lib/seven.js")]);
    assert_eq!(paths.get("7"), Some("lib/seven.js"));
    assert!(paths.contains_key("7"));
    assert_eq!(paths.get("8"), None);
}
