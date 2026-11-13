use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::browserify::notes::{
    LATER_ENTRIES_NOT_RUN, REQUIRE_LEFT_UNBOUND, UNMAPPED_MODULE_ID,
};
use untwist_unpack::{unpack, BundleKind};

const PRELUDE: &str = "(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c=\"function\"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error(\"Cannot find module '\"+i+\"'\");throw a.code=\"MODULE_NOT_FOUND\",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u=\"function\"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()";

struct Split {
    files: Vec<(String, String)>,
    notes: Vec<String>,
}

impl Split {
    fn code(&self, path: &str) -> &str {
        self.files
            .iter()
            .find(|(at, _)| at == path)
            .map(|(_, code)| code.trim_end())
            .unwrap_or_else(|| {
                panic!(
                    "no {path} in {:?}",
                    self.files.iter().map(|(p, _)| p).collect::<Vec<_>>()
                )
            })
    }

    fn notes(&self, text: &str) -> bool {
        self.notes.iter().any(|note| note.contains(text))
    }
}

fn split(arguments: &str) -> Split {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(&format!("{PRELUDE}({arguments});"));
    let parsed = parse_program(&alloc, source);
    let result = unpack(&alloc, &parsed.program, &[]).expect("a browserify bundle");
    assert_eq!(result.bundle.kind, BundleKind::Browserify);
    Split {
        files: result
            .bundle
            .modules
            .values()
            .map(|module| {
                (
                    module.path.as_str().to_string(),
                    pretty_codegen(module.program()),
                )
            })
            .collect(),
        notes: result.notes.into_iter().map(|note| note.message).collect(),
    }
}

#[test]
fn a_mapped_request_names_the_file_of_the_module_its_map_gives() {
    let split = split(
        "{1:[function(require,module,exports){require(\"dep\");require(\"./a\");require(\"events\")},{\"dep\":4,\"./a\":2,\"events\":6}],2:[function(require,module,exports){require(\"dep\")},{\"dep\":5}],4:[function(require,module,exports){},{}],5:[function(require,module,exports){},{}],6:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"dep/index.js\");\nrequire(\"./a.js\");\nrequire(\"events/index.js\");"
    );
    assert_eq!(split.code("a.js"), "require(\"dep/index-2.js\");");
    assert!(
        split
            .notes
            .iter()
            .all(|note| note.contains("output path collision")),
        "{:?}",
        split.notes
    );
}

#[test]
fn a_module_reached_by_two_requests_is_one_file_for_both() {
    let split = split(
        "{1:[function(require,module,exports){require(\"./lib/a\");require(\"./lib/shared\")},{\"./lib/a\":2,\"./lib/shared\":3}],2:[function(require,module,exports){require(\"./shared.js\")},{\"./shared.js\":3}],3:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"./lib/a.js\");\nrequire(\"./lib/shared.js\");"
    );
    assert_eq!(split.code("lib/a.js"), "require(\"./shared.js\");");
}

#[test]
fn a_local_require_and_a_computed_or_unmapped_request_stay_as_written() {
    let split = split(
        "{1:[function(require,module,exports){function load(require){return require(\"./a\")}var name=\"./a\";require(name);require(\"fs\");load(function(){});require(\"./a\")},{\"./a\":2}],2:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "function load(require) {\n  return require(\"./a\");\n}\nvar name = \"./a\";\nrequire(name);\nrequire(\"fs\");\nload(function() {});\nrequire(\"./a.js\");"
    );
    assert!(split.notes.is_empty(), "{:?}", split.notes);
}

#[test]
fn a_request_mapped_to_an_id_the_bundle_does_not_hold_stays_as_written() {
    let split = split(
        "{1:[function(require,module,exports){require(\"./gone\");require(\"./zero\")},{\"./gone\":9,\"./zero\":0}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"./gone\");\nrequire(\"./zero\");"
    );
}

#[test]
fn a_factory_that_names_its_require_differently_links_through_the_hosts() {
    let split = split(
        "{1:[function(t,e,n){var host=typeof require;t(\"./a\")},{\"./a\":2}],2:[function(t,e,n){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "var host = typeof require;\nrequire(\"./a.js\");"
    );
}

#[test]
fn a_request_outside_the_map_that_names_a_module_id_links_to_that_module() {
    let split = split(
        "{main:[function(require,module,exports){require(\"helper\");require(4);require(\"4\");require(4.0);require(\"fs\")},{\"other\":4,\"skipped\":undefined}],helper:[function(require,module,exports){require(\"skipped\")},{\"skipped\":undefined}],4:[function(require,module,exports){},{}],skipped:[function(require,module,exports){},{}]},{},[\"main\"]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"./helper.js\");\nrequire(\"other/index.js\");\nrequire(\"other/index.js\");\nrequire(\"other/index.js\");\nrequire(\"fs\");"
    );
    assert_eq!(split.code("helper.js"), "require(\"./skipped.js\");");
    assert!(split.notes.is_empty(), "{:?}", split.notes);
}

#[test]
fn a_numeric_request_in_the_map_links_like_its_string_form() {
    let split = split(
        "{1:[function(require,module,exports){require(4);require(\"4\")},{\"4\":2}],2:[function(require,module,exports){},{}],4:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"4/index.js\");\nrequire(\"4/index.js\");"
    );
}

#[test]
fn a_request_the_map_inherits_from_object_prototype_stays_as_written() {
    let split = split(
        "{1:[function(require,module,exports){require(\"toString\");require(\"constructor\");require(\"__proto__\")},{}],toString:[function(require,module,exports){},{}],constructor:[function(require,module,exports){},{}],__proto__:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(
        split.code("index.js"),
        "require(\"toString\");\nrequire(\"constructor\");\nrequire(\"__proto__\");"
    );
}

#[test]
fn a_request_mapped_to_a_falsy_id_that_names_a_module_id_is_noted() {
    let split = split(
        "{main:[function(require,module,exports){require(\"helper\")},{\"helper\":0}],helper:[function(require,module,exports){},{}]},{},[\"main\"]",
    );
    assert_eq!(split.code("index.js"), "require(\"helper\");");
    assert!(split.notes(UNMAPPED_MODULE_ID), "{:?}", split.notes);
}

#[test]
fn the_entry_file_runs_every_later_entry_in_order() {
    let split = split(
        "{1:[function(require,module,exports){console.log(1)},{}],7:[function(require,module,exports){console.log(7)},{}],8:[function(require,module,exports){console.log(8)},{}]},{},[1,8,7,8,1]",
    );
    assert_eq!(
        split.code("index.js"),
        "console.log(1);\nrequire(\"./8.js\");\nrequire(\"./7.js\");"
    );
    assert!(split.notes.is_empty(), "{:?}", split.notes);
}

#[test]
fn later_entries_that_cannot_be_run_from_the_entry_file_are_noted() {
    for arguments in [
        "{1:[function(require,module,exports){if(module.parent)return;console.log(1)},{}],7:[function(require,module,exports){},{}]},{},[1,7]",
        "{1:[function(require,module,exports){var require=null;console.log(1)},{}],7:[function(require,module,exports){},{}]},{},[1,7]",
        "{1:[function(require,module,exports){console.log(1)},{}],7:[function(require,module,exports){},{}]},{},[1,9]",
        "{1:[function(require,module,exports){console.log(1)},{}],7:[function(require,module,exports){},{}]},{},[1,later]",
    ] {
        let split = split(arguments);
        assert!(!split.code("index.js").contains("7.js"), "{arguments}");
        assert!(split.notes(LATER_ENTRIES_NOT_RUN), "{arguments}: {:?}", split.notes);
    }
}

#[test]
fn a_return_inside_a_nested_function_does_not_stop_the_entry() {
    let split = split(
        "{1:[function(require,module,exports){exports.f=function(){return 1}},{}],7:[function(require,module,exports){},{}]},{},[1,7]",
    );
    assert!(split.code("index.js").ends_with("require(\"./7.js\");"));
}

#[test]
fn a_factory_that_takes_no_require_keeps_the_hosts_requests() {
    let split = split(
        "{1:[function(){require(\"./a\")},{\"./a\":2}],2:[function(require,module,exports){},{}]},{},[1]",
    );
    assert_eq!(split.code("index.js"), "require(\"./a\");");
}

#[test]
fn a_renamed_require_left_with_a_computed_request_is_noted() {
    let split = split(
        "{1:[function(t,e,n){var host=typeof require;var name=\"./a\";t(name);t(\"./a\")},{\"./a\":2}],2:[function(t,e,n){},{}]},{},[1]",
    );
    assert!(split.code("index.js").ends_with("require(\"./a.js\");"));
    assert!(split.notes(REQUIRE_LEFT_UNBOUND), "{:?}", split.notes);
}

#[test]
fn a_renamed_require_that_is_fully_linked_is_not_noted() {
    let split = split(
        "{1:[function(t,e,n){var host=typeof require;t(\"./a\")},{\"./a\":2}],2:[function(t,e,n){},{}]},{},[1]",
    );
    assert!(!split.notes(REQUIRE_LEFT_UNBOUND), "{:?}", split.notes);
}

fn refusal(arguments: &str) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(&format!("{PRELUDE}({arguments});"));
    let parsed = parse_program(&alloc, source);
    let mut programs = vec![parsed.program];
    let (bundle, notes) = untwist_unpack::unpack_inputs(
        &alloc,
        &mut programs,
        &[],
        untwist_unpack::UnpackOptions::default(),
    );
    bundle
        .is_none()
        .then(|| notes.into_iter().map(|note| note.message).collect())
}

#[test]
fn a_module_that_redeclares_its_require_parameter_refuses_the_bundle() {
    let notes = refusal(
        "{1:[function(e,t,n){var e=e(\"./a\");t.exports=e},{\"./a\":2}],2:[function(e,t,n){t.exports=1},{}]},{},[1]",
    )
    .expect("the bundle is refused");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("module 1") && note.contains("no bundle is written")),
        "{notes:?}"
    );
}

#[test]
fn a_module_that_assigns_its_require_parameter_refuses_the_bundle() {
    assert!(refusal(
        "{1:[function(e,t,n){var a=e(\"./a\");e=null;t.exports=a},{\"./a\":2}],2:[function(e,t,n){t.exports=1},{}]},{},[1]",
    )
    .is_some());
}

#[test]
fn a_module_that_only_calls_its_require_parameter_is_unpacked() {
    assert!(refusal(
        "{1:[function(e,t,n){var a=e(\"./a\");t.exports=a},{\"./a\":2}],2:[function(e,t,n){t.exports=1},{}]},{},[1]",
    )
    .is_none());
}

#[test]
fn a_require_parameter_named_require_may_be_declared_again() {
    assert!(refusal(
        "{1:[function(require,module,exports){var require=require(\"./a\");module.exports=require},{\"./a\":2}],2:[function(require,module,exports){module.exports=1},{}]},{},[1]",
    )
    .is_none());
}
