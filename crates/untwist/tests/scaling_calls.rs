mod common;

use common::scaling::assert_scales_linearly;
use untwist::Options;

fn webpack_chain(modules: usize) -> String {
    let factories: Vec<String> = (0..modules)
        .map(|index| {
            let body = if index + 1 < modules {
                format!("var m=__webpack_require__({});module.exports=m;", index + 1)
            } else {
                "module.exports=1;".to_owned()
            };
            format!("function(module,exports,__webpack_require__){{{body}}}")
        })
        .collect();
    format!(
        "(function(modules){{var installedModules={{}};function __webpack_require__(moduleId){{\
         if(installedModules[moduleId]){{return installedModules[moduleId].exports}}\
         var module=installedModules[moduleId]={{i:moduleId,l:false,exports:{{}}}};\
         modules[moduleId].call(module.exports,module,module.exports,__webpack_require__);\
         module.l=true;return module.exports}}__webpack_require__.m=modules;\
         __webpack_require__.c=installedModules;\
         return __webpack_require__(__webpack_require__.s=0)}})([{}]);",
        factories.join(",")
    )
}

#[test]
fn a_counter_used_as_a_key_after_every_write_is_judged_once() {
    assert_scales_linearly(
        "i=i+1;q[i];",
        1_000,
        |count| format!("var i=0;{}", "i=i+1;q[i];".repeat(count)),
        Options::default,
    );
}

#[test]
fn a_reused_loop_variable_is_judged_once() {
    assert_scales_linearly(
        "for(var i=0;i<9;i++){a(i)}",
        1_000,
        |count| "for(var i=0;i<9;i++){a(i)}".repeat(count),
        Options::default,
    );
}

#[test]
fn two_variables_written_from_each_other_are_judged_once() {
    assert_scales_linearly(
        "a=b;b=a;q[a];",
        250,
        |count| format!("var a=1,b=2;{}", "a=b;b=a;q[a];".repeat(count)),
        Options::default,
    );
}

#[test]
fn calls_of_one_free_global_look_its_writes_up_once() {
    assert_scales_linearly(
        "a();",
        2_500,
        |count| {
            format!(
                "function w(a,b){{return a+b}}x=w(1,2);{}",
                "a();".repeat(count)
            )
        },
        Options::default,
    );
}

#[test]
fn every_require_of_a_webpack_bundle_finds_its_module_id_by_lookup() {
    assert_scales_linearly(
        "webpack 4 require chain",
        1_000,
        webpack_chain,
        Options::default,
    );
}
