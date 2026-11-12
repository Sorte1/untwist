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

const DECODER_HEAD: &str = "function arr(){var h=['gol','olleh','dlrow','cba','fed'];arr=function(){return h};return arr()}\
function dec(a,b){a=a-100;var e=arr();return e[a]}console.log(dec(101));";

#[test]
fn one_variable_given_a_new_object_in_every_block_is_judged_once() {
    assert_scales_linearly(
        "o={};o.a=1;list.push(o);",
        2_000,
        |count| format!("var o;{}", "o={};o.a=1;o.b=2;list.push(o);".repeat(count)),
        Options::default,
    );
}

#[test]
fn fresh_writes_with_computed_keys_beside_a_decoder_are_judged_once() {
    assert_scales_linearly(
        "v={};v[w]=1;",
        2_000,
        |count| format!("{DECODER_HEAD}var v={{}};{}", "v={};v[w]=1;".repeat(count)),
        Options::default,
    );
}

#[test]
fn method_calls_on_one_binding_beside_a_decoder_look_its_writes_up_once() {
    assert_scales_linearly(
        "list.push(1);",
        2_000,
        |count| {
            format!(
                "{DECODER_HEAD}var list=[];{}",
                "list.push(1);".repeat(count)
            )
        },
        Options::default,
    );
}

#[test]
fn a_parameter_called_at_many_sites_beside_a_decoder_looks_its_writes_up_once() {
    assert_scales_linearly(
        "p.call(1);",
        1_000,
        |count| {
            format!(
                "{DECODER_HEAD}function f(p){{{}}}{}",
                "p.call(1);".repeat(count),
                "f(function(){});".repeat(count)
            )
        },
        Options::default,
    );
}
