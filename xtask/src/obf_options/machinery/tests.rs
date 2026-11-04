use super::shapes::printable_escapes;
use super::*;

const STRING_ARRAY: &str = "function _0xd3f4(){var a=['log','hi'];_0xd3f4=function(){return a;};return _0xd3f4();}\
    function _0xcdc4(b,c){b=b-0xaa;var d=_0xd3f4();return d[b];}var _0x4cbe=_0xcdc4;\
    function _0xw(e,f){return _0xcdc4(f-0x1,e);}\
    (function(g,h){var i=g();while(!![]){try{if(1)break;else i['push'](i['shift']());}catch(j){i['push'](i['shift']());}}}(_0xd3f4,0x1));\
    console[_0x4cbe(0xb2)](_0xw(0x1,0xb3));";

#[test]
fn string_array_machinery_is_counted() {
    let found = measure(STRING_ARRAY);
    assert!(found.parses);
    assert_eq!(found.get("string_arrays"), 1);
    assert_eq!(found.get("decoders"), 1);
    assert_eq!(found.get("rotators"), 2);
    assert_eq!(found.get("decoder_calls"), 2 + 1);
    assert!(found.get("hex_numbers") > 0);
    assert!(found.get("bool_arrays") > 0);
    assert!(found.get("computed_members") > 0);
}

#[test]
fn hostile_machinery_is_counted() {
    let found = measure(
        "var c=(function(){var f=!![];return function(x,g){var r=f?function(){if(g){var s=g['apply'](x,arguments);return g=null,s;}}:function(){};return f=![],r;};}());\
         function d(n){(function(){}['constructor']('debu'+'gger')['call']('action'));debugger;}\
         var s='x'['search']('(((.+)+)+)+$');var m=['log','warn','info','error','exception','table','trace'];\
         var g=Function('return (function() '+'{}.constructor(\"return this\")( )'+');');\
         var h='wabout:blankw'['replace'](new RegExp('[w]','g'),'');for(var k in w){if(q(k,0x8,[0x7,0x74,0x5,0x65])){}}",
    );
    for class in [
        "call_controllers",
        "debug_protection",
        "self_defending",
        "console_override",
        "global_getters",
        "domain_lock",
        "split_strings",
    ] {
        assert!(found.get(class) > 0, "{class} in {found:?}");
    }
}

#[test]
fn flattening_dead_code_and_expression_shapes_are_counted() {
    let found = measure(
        "var o={'BqxrS':function(a,b){return a+b;},'NOsnk':'hello'};var p='3|1|0'['split']('|'),q=0;\
         while(!![]){switch(p[q++]){case'0':continue;}break;}\
         if('abc'==='xyz'){}var n=-0x2*0x107b+0x4161;var t='\\x68\\x69';var u={};u['k']=1;var v=u;\
         function f(){var junk;var used;used=1;return used;}",
    );
    for class in [
        "proxy_objects",
        "dispatchers",
        "opaque_predicates",
        "number_expressions",
        "escapes",
        "object_key_assignments",
        "quoted_keys",
        "dead_declarations",
    ] {
        assert!(found.get(class) > 0, "{class} in {found:?}");
    }
}

#[test]
fn plain_code_measures_clean() {
    let found = measure(
        "const words = ['a', 'b']; class A { describe() { return 'x\\n\\t\\\\x41' + words[0]; } }\
         const o = { width: 6, pears: 7, ...words }; console.log(o.width * 2, \"caf\u{e9}\", 3 - 1 > 0 ? -1 : 1);\
         for (const k in o) { keys.push(k); } let tally = {}; tally = { count: 0 };",
    );
    assert!(found.parses);
    let classes: Vec<&str> = MACHINERY.iter().chain(SIMPLIFY.iter()).copied().collect();
    assert_eq!(found.render(&classes), "number_expressions=1");
}

#[test]
fn escapes_skip_escaped_backslashes_and_non_ascii() {
    assert_eq!(printable_escapes("'\\\\x41'"), 0);
    assert_eq!(printable_escapes("'\\x41\\u0042\\u{43}'"), 3);
    assert_eq!(printable_escapes("'\\u00e9\\x0a'"), 0);
}
