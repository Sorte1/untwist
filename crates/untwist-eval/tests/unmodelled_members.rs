use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{record_units_charged, UnitRecord};
use untwist_eval::unmodelled::{installed_members, standard_members, uninstalled_members};
use untwist_testing::node::printed_by_node;

fn units(source: &str, decoding: bool) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{source}");
    record_units_charged(|| match decoding {
        true => eval_script_decoding(&parsed.program, Budget::default(), |_| ()),
        false => eval_script_in(&parsed.program, Budget::default(), |_, _| ()),
    })
}

fn call(body: &str) -> String {
    format!("String((function () {{ {body} }})())")
}

fn answers_as_node_or_bails(rows: &[(&str, &str)], must_answer: bool) {
    let mut wrong = Vec::new();
    for (body, node) in rows {
        if let Some(printed) = printed_by_node(&format!("console.log({});", call(body))) {
            assert_eq!(printed.trim_end(), *node, "node on {body}");
        }
        for decoding in [false, true] {
            let unit = &units(&format!("{};", call(body)), decoding)[0];
            let answered = unit.values == [format!("string:{node}")];
            let bailed = unit.bail.is_some();
            let fine = if must_answer && !decoding {
                answered
            } else {
                answered || bailed
            };
            if !fine {
                wrong.push(format!("decoding={decoding} {unit:?}: {body}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

const PLACEHOLDER_CONSTRUCTORS: &[(&str, &str)] = &[
    (
        "try { null.x } catch (e) { return typeof e.constructor }",
        "function",
    ),
    ("return typeof ({}).constructor", "function"),
    ("return ({}).constructor === Object", "true"),
    ("return typeof [].constructor", "function"),
    ("return typeof Object.prototype.constructor", "function"),
    ("return typeof /a/.constructor", "function"),
    ("return typeof (1).constructor", "function"),
];

#[test]
fn a_constructor_the_realm_does_not_model_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(PLACEHOLDER_CONSTRUCTORS, false);
}

const HOLES_UNDER_AN_INDEXED_PROTOTYPE: &[(&str, &str)] = &[
    (
        "Object.prototype[0] = 'o'; var a = [, 1], i = 0; return a[i]",
        "o",
    ),
    ("Object.prototype[0] = 'o'; var a = [, 1]; return a[0]", "o"),
    (
        "Object.prototype[0] = 'o'; var a = []; a.length = 2; return a[0]",
        "o",
    ),
    (
        "Object.prototype[0] = 'o'; var a = [1, 2]; delete a[0]; return a[0]",
        "o",
    ),
    (
        "Object.prototype[0] = 'o'; var a = [undefined, 1]; return a[0]",
        "undefined",
    ),
];

#[test]
fn a_hole_read_under_an_indexed_prototype_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(HOLES_UNDER_AN_INDEXED_PROTOTYPE, false);
}

const READS_THROUGH_A_WRITTEN_PROTOTYPE: &[(&str, &str)] = &[
    ("Object.prototype[0] = 'o'; return [, 1].join()", "o,1"),
    ("Object.prototype[0] = 'o'; return [, 1].indexOf('o')", "0"),
    (
        "var a = [104, 105]; Object.prototype[2] = 106; a.length = 3; return String.fromCharCode.apply(null, a)",
        "hij",
    ),
    (
        "Object.prototype['0'] = 'o'; var r = ''; for (var k in {a: 1}) r += k; return r",
        "a0",
    ),
    ("var a = []; a.__proto__ = {x: 1}; return a.x", "1"),
    ("var o = {}; o['__proto__'] = {x: 1}; return o.x", "1"),
    ("var o = {__proto__: {x: 1}}; return o.x", "1"),
    (
        "var o = Object.assign({}, JSON.parse('{\"__proto__\": {\"x\": 1}}')); return o.x",
        "1",
    ),
];

#[test]
fn a_read_through_a_prototype_the_program_wrote_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(READS_THROUGH_A_WRITTEN_PROTOTYPE, false);
}

const HOLES: &[(&str, &str)] = &[
    ("return Object.keys([, 1])", "1"),
    ("return 0 in [, 1]", "false"),
    ("var a = [, 1]; return a[0]", "undefined"),
    ("var n = 0; [, 1].forEach(function () { n++ }); return n", "1"),
    ("var a = []; a[3] = 'x'; return Object.keys(a)", "3"),
    ("var a = [1]; a.length = 3; return 1 in a", "false"),
    ("var a = [1, 2]; delete a[0]; return Object.keys(a)", "1"),
    (
        "function f(a, b) { arguments[5] = 7; return arguments.length } return f(1, 2)",
        "2",
    ),
    (
        "function f() { arguments.length = 5; return Object.keys(arguments).length } return f(1, 2)",
        "2",
    ),
    (
        "function f(a, b) { delete arguments[0]; return Object.keys(arguments) } return f(1, 2)",
        "1",
    ),
    (
        "function f() { delete arguments[0]; return 0 in arguments } return f(1, 2)",
        "false",
    ),
];

#[test]
fn a_hole_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(HOLES, false);
}

const UNMODELLED_OUTSIDE_DECODING: &[(&str, &str)] = &[
    (
        "return String.fromCharCode.apply(null, {length: 2, 0: 104, 1: 105})",
        "hi",
    ),
    ("return typeof [].at", "function"),
    ("return typeof ''.at", "function"),
    ("return typeof /a/.hasIndices", "boolean"),
    ("return typeof (1).toLocaleString", "function"),
    ("return [1, 2].at(-1)", "2"),
    ("return [1, 2].lastIndexOf(2)", "1"),
    ("return Object.getOwnPropertyNames([1]).length", "2"),
    (
        "var a = []; Object.setPrototypeOf(a, {x: 1}); return a.x",
        "1",
    ),
    ("return globalThis.eval.call(null, '1')", "1"),
    (
        "return parseInt.toString()",
        "function parseInt() { [native code] }",
    ),
    (
        "return String(parseInt)",
        "function parseInt() { [native code] }",
    ),
    (
        "function f() { return typeof this } return f.call(1)",
        "object",
    ),
    (
        "return (function () { return this === undefined })()",
        "false",
    ),
    ("return globalThis === this", "true"),
    ("var x = 1; return delete x", "false"),
    ("return typeof true.toString", "function"),
    (
        "try { null.x } catch (e) { return e.message }",
        "Cannot read properties of null (reading 'x')",
    ),
    ("try { null.x } catch (e) { return e.name }", "TypeError"),
    (
        "try { null.x } catch (e) { return String(e) }",
        "TypeError: Cannot read properties of null (reading 'x')",
    ),
    ("var re = /a/g; re.test('aa'); return re.lastIndex", "1"),
    (
        "function f() {} return String(f.bind(null))",
        "function () { [native code] }",
    ),
    ("return globalThis.parseInt === parseInt", "true"),
    ("return 'parseInt' in globalThis", "true"),
    ("globalThis.z = 1; return typeof z", "number"),
    (
        "var o = Object.create({a: 1}); var r = ''; for (var k in o) r += k; return r",
        "a",
    ),
];

#[test]
fn an_unmodelled_member_native_or_this_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(UNMODELLED_OUTSIDE_DECODING, false);
}

const STILL_ANSWERED: &[(&str, &str)] = &[
    ("var a = [undefined, 1], i = 0; return a[i]", "undefined"),
    ("var a = [1]; a[1] = 2; return a.join()", "1,2"),
    ("var a = [1, 2, 3]; a.length = 1; return a.join()", "1"),
    ("var o = {}; o.x = 1; return Object.keys(o)", "x"),
    (
        "function f(a, b) { arguments[1] = 7; return arguments.length + ':' + b } return f(1, 2)",
        "2:7",
    ),
    (
        "'use strict'; function f() { return typeof this } return f.call(1)",
        "number",
    ),
    (
        "var o = {f: function () { return typeof this }}; return o.f()",
        "object",
    ),
    ("return typeof Object.keys", "function"),
    ("return (function () {}).toString()", "function () {}"),
    ("return [1, 2].indexOf(2)", "1"),
    ("return typeof ({}).foo", "undefined"),
    ("return typeof 'ab'.foo", "undefined"),
    (
        "var p = Object.getPrototypeOf([]); delete p.join; return typeof [].join",
        "undefined",
    ),
    (
        "var p = Object.getPrototypeOf(''); delete p.trim; return typeof ''.trim",
        "undefined",
    ),
    ("var a = () => 1; return typeof a.prototype", "undefined"),
    (
        "function f() {} return typeof f.bind(null).prototype",
        "undefined",
    ),
    ("return typeof ({m() {}}).m.prototype", "undefined"),
    ("return typeof 'a'.constructor", "function"),
    ("return typeof (function () {}).constructor", "function"),
];

#[test]
fn a_hole_or_a_constructor_with_nothing_to_inherit_still_answers_outside_decoding() {
    answers_as_node_or_bails(STILL_ANSWERED, true);
}

const ES_ARRAY_PROTOTYPE: [&str; 40] = [
    "length",
    "constructor",
    "at",
    "concat",
    "copyWithin",
    "fill",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "lastIndexOf",
    "pop",
    "push",
    "reverse",
    "shift",
    "unshift",
    "slice",
    "sort",
    "splice",
    "includes",
    "indexOf",
    "join",
    "keys",
    "entries",
    "values",
    "forEach",
    "filter",
    "flat",
    "flatMap",
    "map",
    "every",
    "some",
    "reduce",
    "reduceRight",
    "toReversed",
    "toSorted",
    "toSpliced",
    "with",
    "toLocaleString",
    "toString",
];

const ES_STRING_PROTOTYPE: [&str; 52] = [
    "length",
    "constructor",
    "anchor",
    "at",
    "big",
    "blink",
    "bold",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "concat",
    "endsWith",
    "fontcolor",
    "fontsize",
    "fixed",
    "includes",
    "indexOf",
    "isWellFormed",
    "italics",
    "lastIndexOf",
    "link",
    "localeCompare",
    "match",
    "matchAll",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "replace",
    "replaceAll",
    "search",
    "slice",
    "small",
    "split",
    "strike",
    "sub",
    "substr",
    "substring",
    "sup",
    "startsWith",
    "toString",
    "toWellFormed",
    "trim",
    "trimStart",
    "trimLeft",
    "trimEnd",
    "trimRight",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "toLowerCase",
    "toUpperCase",
    "valueOf",
];

const ES_NUMBER_PROTOTYPE: [&str; 7] = [
    "constructor",
    "toExponential",
    "toFixed",
    "toPrecision",
    "toString",
    "valueOf",
    "toLocaleString",
];

const ES_REGEXP_PROTOTYPE: [&str; 15] = [
    "constructor",
    "exec",
    "dotAll",
    "flags",
    "global",
    "hasIndices",
    "ignoreCase",
    "multiline",
    "source",
    "sticky",
    "unicode",
    "compile",
    "toString",
    "test",
    "unicodeSets",
];

const ES_FUNCTION_PROTOTYPE: [&str; 5] = ["constructor", "apply", "bind", "call", "toString"];

const STANDARD: [(&str, &[&str]); 5] = [
    ("Array.prototype", &ES_ARRAY_PROTOTYPE),
    ("Function.prototype", &ES_FUNCTION_PROTOTYPE),
    ("Number.prototype", &ES_NUMBER_PROTOTYPE),
    ("RegExp.prototype", &ES_REGEXP_PROTOTYPE),
    ("String.prototype", &ES_STRING_PROTOTYPE),
];

#[test]
fn the_standard_member_list_is_the_realms_install_table_plus_the_spec_remainder() {
    for (owner, spec) in STANDARD {
        let installed = installed_members(owner);
        let uninstalled = uninstalled_members(owner);
        let both: Vec<_> = uninstalled
            .iter()
            .filter(|name| installed.contains(name))
            .collect();
        assert!(
            both.is_empty(),
            "{owner}: installed and listed as uninstalled: {both:?}"
        );
        let unlisted: Vec<_> = installed
            .iter()
            .filter(|name| !spec.contains(name))
            .collect();
        assert!(
            unlisted.is_empty(),
            "{owner}: installed but not a spec member: {unlisted:?}"
        );
        let mut derived = standard_members(owner);
        derived.sort_unstable();
        let mut expected = spec.to_vec();
        expected.sort_unstable();
        assert_eq!(derived, expected, "{owner}");
    }
}

const UNINSTALLED_MISSES: &[(&str, &str)] = &[
    ("return typeof ''.normalize", "function"),
    ("return typeof [].flat", "function"),
    ("return typeof /a/.flags", "string"),
    ("return typeof (1).toPrecision", "function"),
    ("return typeof [].at", "function"),
];

#[test]
fn a_miss_on_an_uninstalled_standard_member_bails_while_decoding() {
    for (body, node) in UNINSTALLED_MISSES {
        if let Some(printed) = printed_by_node(&format!("console.log({});", call(body))) {
            assert_eq!(printed.trim_end(), *node, "node on {body}");
        }
        let unit = &units(&format!("{};", call(body)), true)[0];
        assert!(unit.bail.is_some(), "{body}: {unit:?}");
    }
}

const OWN_MISSES: &[(&str, &str)] = &[
    (
        "try { null.x } catch (e) { return e.hasOwnProperty('message') }",
        "true",
    ),
    (
        "try { null.x } catch (e) { return Object.prototype.hasOwnProperty.call(e, 'message') }",
        "true",
    ),
    (
        "try { [].reduce(function () {}) } catch (e) { return e.hasOwnProperty('message') }",
        "true",
    ),
    (
        "try { null.x } catch (e) { return e.hasOwnProperty('stack') }",
        "true",
    ),
    (
        "return Object.prototype.hasOwnProperty.call(String.prototype, 'at')",
        "true",
    ),
    (
        "return Object.prototype.hasOwnProperty.call(Math, 'hypot')",
        "true",
    ),
    (
        "return Object.prototype.hasOwnProperty.call(parseInt, 'length')",
        "true",
    ),
    ("return Math.max.hasOwnProperty('name')", "true"),
    ("return (function () {}).hasOwnProperty('name')", "true"),
    (
        "return (function () {}).bind(null).hasOwnProperty('name')",
        "true",
    ),
    ("return /a/g.hasOwnProperty('lastIndex')", "true"),
    (
        "return (function () { return arguments.hasOwnProperty('callee') })(1)",
        "true",
    ),
    (
        "return (function () { 'use strict'; return arguments.hasOwnProperty('callee') })(1)",
        "true",
    ),
    ("return globalThis.hasOwnProperty('Math')", "true"),
    (
        "return Object.prototype.hasOwnProperty.call(globalThis, 'Math')",
        "true",
    ),
];

#[test]
fn an_own_property_the_realm_does_not_model_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(OWN_MISSES, false);
}

const OWN_MISSES_STILL_ANSWERED: &[(&str, &str)] = &[
    ("return ({a: 1}).hasOwnProperty('a')", "true"),
    ("return ({a: 1}).hasOwnProperty('b')", "false"),
    ("return ({}).hasOwnProperty('toString')", "false"),
    ("return [].hasOwnProperty('push')", "false"),
    ("return [1].hasOwnProperty(0)", "true"),
    ("return 'ab'.hasOwnProperty('length')", "true"),
    ("return 'ab'.hasOwnProperty('at')", "false"),
    (
        "try { null.x } catch (e) { e.foo = 1; return e.hasOwnProperty('foo') }",
        "true",
    ),
];

#[test]
fn an_own_property_of_an_ordinary_object_still_answers_outside_decoding() {
    answers_as_node_or_bails(OWN_MISSES_STILL_ANSWERED, true);
}

const ENGINE_KEPT_DELETES: &[(&str, &str)] = &[
    ("return delete /a/.lastIndex", "false"),
    ("var r = /a/g; delete r.lastIndex; return r.lastIndex", "0"),
    ("return delete String.prototype.length", "false"),
    ("return delete [].length", "false"),
    ("var a = [1, 2]; delete a.length; return a.length", "2"),
    ("return delete Math.PI", "false"),
    (
        "return (function () { delete arguments.length; return arguments.length })(1)",
        "undefined",
    ),
    (
        "return (function () { delete arguments.length; return arguments.length })(1, 2)",
        "undefined",
    ),
    (
        "return (function () { 'use strict'; delete arguments.length; return arguments.length })(1)",
        "undefined",
    ),
    (
        "return (function () { delete arguments.length; return [].slice.call(arguments).length })(1, 2)",
        "0",
    ),
    ("var s = 'ab'; return delete s.length", "false"),
    ("var s = 'ab'; return delete s[0]", "false"),
    ("return delete 'ab'.length", "false"),
    (
        "return (function () { 'use strict'; try { delete 'ab'.length; return 'no' } catch (e) { return e.name } })()",
        "TypeError",
    ),
];

#[test]
fn a_delete_of_a_key_the_engine_keeps_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(ENGINE_KEPT_DELETES, false);
}

const DELETES_STILL_ANSWERED: &[(&str, &str)] = &[
    ("var o = {a: 1}; return delete o.a", "true"),
    ("var o = {a: 1}; delete o.a; return typeof o.a", "undefined"),
    ("var s = 'ab'; return delete s[5]", "true"),
    ("var s = 'ab'; return delete s.foo", "true"),
    ("return delete (5).x", "true"),
    ("return delete /a/.foo", "true"),
];

#[test]
fn a_delete_of_an_ordinary_key_still_answers_outside_decoding() {
    answers_as_node_or_bails(DELETES_STILL_ANSWERED, true);
}

const NAMED_PROTOTYPE_WRITES: &[(&str, &str)] = &[
    (
        "var o = {}; var s = ''; Object.prototype.y = 1; for (var k in o) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in {a: 1}) s += k; return s",
        "ay",
    ),
    (
        "var s = ''; Object.prototype.y = 1; for (var k in [1]) s += k; return s",
        "0y",
    ),
    (
        "var s = ''; String.prototype.y = 1; for (var k in 'a') s += k; return s",
        "0y",
    ),
    (
        "Array.prototype.y = 1; var s = ''; for (var k in [1]) s += k; return s",
        "0y",
    ),
    (
        "Function.prototype.y = 1; var s = ''; for (var k in function () {}) s += k; return s",
        "y",
    ),
];

const NAMED_WRITES_UNDER_EVERY_SUBJECT: &[(&str, &str)] = &[
    (
        "Object.prototype.y = 1; var s = ''; for (var k in 'a') s += k; return s",
        "0y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in 5) s += k; return s",
        "y",
    ),
    (
        "Number.prototype.y = 1; var s = ''; for (var k in 5) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in function () {}) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in () => 1) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in (function () {}).bind(null)) s += k; return s",
        "y",
    ),
    (
        "Function.prototype.y = 1; var s = ''; for (var k in (function () {}).bind(null)) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; return (function () { var s = ''; for (var k in arguments) s += k; return s })(1)",
        "0y",
    ),
    (
        "Array.prototype.y = 1; return (function () { var s = ''; for (var k in arguments) s += k; return s })(1)",
        "0",
    ),
    (
        "Object.prototype.y = 1; try { null.x } catch (e) { var s = ''; for (var k in e) s += k; return s }",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in /a/) s += k; return s",
        "y",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in true) s += k; return s",
        "y",
    ),
];

#[test]
fn a_named_write_to_an_intrinsic_prototype_reaches_every_for_in_subject() {
    answers_as_node_or_bails(NAMED_WRITES_UNDER_EVERY_SUBJECT, false);
}

#[test]
fn a_named_write_to_an_intrinsic_prototype_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(NAMED_PROTOTYPE_WRITES, false);
}

const NAMED_WRITES_STILL_ANSWERED: &[(&str, &str)] = &[
    ("Math.y = 1; return Math.y", "1"),
    ("var o = {}; o.y = 1; return o.y", "1"),
];

#[test]
fn a_named_write_to_an_ordinary_object_still_answers_outside_decoding() {
    answers_as_node_or_bails(NAMED_WRITES_STILL_ANSWERED, true);
}

const NULLISH_FOR_IN_AFTER_A_PROTOTYPE_WRITE: &[(&str, &str)] = &[
    (
        "Object.prototype.y = 1; var s = ''; for (var k in null) s += k; return s",
        "",
    ),
    (
        "Object.prototype.y = 1; var s = ''; for (var k in undefined) s += k; return s",
        "",
    ),
    (
        "Object.prototype.y = 1; var o = null; var s = ''; for (var k in o) s += k; return s + '.'",
        ".",
    ),
];

#[test]
fn a_for_in_over_null_or_undefined_yields_no_keys_after_a_prototype_write() {
    answers_as_node_or_bails(NULLISH_FOR_IN_AFTER_A_PROTOTYPE_WRITE, true);
}

const GLOBAL_AND_NAMESPACE_TAGS: &[(&str, &str)] = &[
    ("return String(globalThis)", "[object global]"),
    ("return globalThis.toString()", "[object global]"),
    ("return globalThis + ''", "[object global]"),
    (
        "return Object.prototype.toString.call(globalThis)",
        "[object global]",
    ),
    (
        "return Object.prototype.toString.call(Math)",
        "[object Math]",
    ),
    (
        "return Object.prototype.toString.call(JSON)",
        "[object JSON]",
    ),
    ("return String(Math)", "[object Math]"),
    ("return Math + ''", "[object Math]"),
    (
        "return Object.keys(Object.assign({}, globalThis)).length > 0",
        "true",
    ),
    (
        "return Object.keys(Object.assign({}, globalThis, {a: 1})).length > 1",
        "true",
    ),
];

#[test]
fn the_global_object_or_a_namespace_as_a_tag_or_an_assign_source_answers_as_node_or_bails() {
    answers_as_node_or_bails(GLOBAL_AND_NAMESPACE_TAGS, false);
}

const TAGS_STILL_ANSWERED: &[(&str, &str)] = &[
    ("return String({})", "[object Object]"),
    (
        "return Object.prototype.toString.call([])",
        "[object Array]",
    ),
    (
        "return Object.prototype.toString.call(parseInt)",
        "[object Function]",
    ),
    (
        "return Object.keys(Object.assign({}, {a: 1}, {b: 2})).join()",
        "a,b",
    ),
];

#[test]
fn a_tag_or_an_assign_of_an_ordinary_object_still_answers_outside_decoding() {
    answers_as_node_or_bails(TAGS_STILL_ANSWERED, true);
}

const UNDECLARED_TYPEOF: &[(&str, &str)] = &[
    ("return typeof window", "undefined"),
    ("return typeof document", "undefined"),
    ("return typeof global", "object"),
    ("return typeof Symbol", "function"),
    ("return typeof Map", "function"),
    ("return typeof Reflect", "object"),
    ("return typeof structuredClone", "function"),
    ("return typeof module", "object"),
];

#[test]
fn typeof_an_undeclared_or_hostile_global_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(UNDECLARED_TYPEOF, false);
}

const TYPEOF_STILL_ANSWERED: &[(&str, &str)] = &[
    ("return typeof Math", "object"),
    ("return typeof parseInt", "function"),
    ("return typeof globalThis", "object"),
    ("var x; return typeof x", "undefined"),
    ("return typeof undefined", "undefined"),
];

#[test]
fn typeof_a_declared_name_still_answers_outside_decoding() {
    answers_as_node_or_bails(TYPEOF_STILL_ANSWERED, true);
}

#[test]
fn every_global_node_binds_is_one_typeof_bails_on() {
    let listing = "console.log(Object.getOwnPropertyNames(globalThis).concat(['require', 'module', 'exports', '__filename', '__dirname']).join('\\n'))";
    let Some(printed) = printed_by_node(listing) else {
        return;
    };
    let missing: Vec<&str> = printed
        .lines()
        .filter(|name| !untwist_eval::unmodelled::is_node_global(name))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

const ARGUMENTS_IN_A_PARAMETER_DEFAULT: &[(&str, &str)] = &[
    (
        "function f(a, b = arguments.length) { return b } return f(1)",
        "1",
    ),
    (
        "function f(a, b = arguments[0]) { return b } return f(5)",
        "5",
    ),
    (
        "function f(a, g = () => arguments.length) { return g() } return f(1, undefined, 3)",
        "3",
    ),
    (
        "function f(a = 1) { return arguments.length } return f()",
        "0",
    ),
];

#[test]
fn arguments_read_in_a_parameter_default_answers_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(ARGUMENTS_IN_A_PARAMETER_DEFAULT, false);
}

const A_BODY_VAR_SHADOWING_A_PARAMETER: &[(&str, &str)] = &[
    (
        "function f(a, g = function () { return a }) { var a = 2; return g() } return f(1)",
        "1",
    ),
    (
        "function f(a, g = () => a) { var a = 2; return g() + ':' + a } return f(1)",
        "1:2",
    ),
    (
        "function f(a, g = () => a) { var a; return g() + ':' + a } return f(1)",
        "1:1",
    ),
    (
        "function f(a, g = function () { a = 3 }) { var a = 2; g(); return a } return f(1)",
        "2",
    ),
    ("function f(a = 1) { var a; return a } return f()", "1"),
];

#[test]
fn a_closure_in_a_parameter_default_over_a_shadowing_body_var_answers_as_node_or_bails() {
    answers_as_node_or_bails(A_BODY_VAR_SHADOWING_A_PARAMETER, false);
}

const A_FUNCTION_PROTOTYPE_CONSTRUCTOR: &[(&str, &str)] = &[
    (
        "function f() {} return f.prototype.hasOwnProperty('constructor')",
        "true",
    ),
    (
        "function f() {} return Object.prototype.hasOwnProperty.call(f.prototype, 'constructor')",
        "true",
    ),
    (
        "function f() {} return f.prototype.constructor === f",
        "true",
    ),
    (
        "function f() {} return Object.keys(f.prototype).length",
        "0",
    ),
    (
        "function f() {} var s = ''; for (var k in f.prototype) s += k; return s + '.'",
        ".",
    ),
    (
        "function f() {} return Object.getOwnPropertyNames(f.prototype).join()",
        "constructor",
    ),
    ("function f() {} return new f().constructor === f", "true"),
    (
        "function f() {} return new f().hasOwnProperty('constructor')",
        "false",
    ),
    (
        "function f() {} return 'constructor' in f.prototype",
        "true",
    ),
    ("function* g() {} return typeof g.prototype", "object"),
    (
        "function* g() {} return g.prototype.hasOwnProperty('constructor')",
        "false",
    ),
    (
        "function* g() {} return Object.keys(g.prototype).length",
        "0",
    ),
];

#[test]
fn a_function_prototype_has_its_own_constructor_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(A_FUNCTION_PROTOTYPE_CONSTRUCTOR, false);
}

const NEW_ON_A_BOUND_FUNCTION: &[(&str, &str)] = &[
    (
        "function A() {} A.prototype.k = 1; return new (A.bind(null))().k",
        "1",
    ),
    (
        "function A() {} var B = A.bind(null); return new B() instanceof B",
        "true",
    ),
    (
        "function A() {} var B = A.bind(null); return new B() instanceof A",
        "true",
    ),
    (
        "function A(x, y) { this.s = x + y } var B = A.bind(null, 3); return new B(4).s",
        "7",
    ),
    (
        "function A() {} var B = A.bind(null); return Object.getPrototypeOf(new B()) === A.prototype",
        "true",
    ),
    (
        "function A() {} var o = {}; return o instanceof A.bind(null)",
        "false",
    ),
    (
        "function A(x, y) { this.s = x + ':' + y } var B = A.bind(null, 1).bind(null, 2); return new B().s",
        "1:2",
    ),
    (
        "function A(x) { this.x = x } var B = A; for (var i = 0; i < 2000; i++) B = B.bind(null); return new B(5).x",
        "5",
    ),
    (
        "var B = (() => 1).bind(null); try { new B(); return 'no' } catch (e) { return e.name }",
        "TypeError",
    ),
];

#[test]
fn new_on_a_bound_function_uses_the_target_prototype_as_node_or_bails_in_both_modes() {
    answers_as_node_or_bails(NEW_ON_A_BOUND_FUNCTION, false);
}

const ANNEX_B_BLOCK_FUNCTIONS: &[(&str, &str)] = &[
    ("{ function h() {} } return typeof h", "function"),
    ("{ function h() { return 1 } } return h()", "1"),
    ("if (true) { function h() {} } return typeof h", "function"),
    ("if (false) { function h() {} } return typeof h", "undefined"),
    (
        "switch (1) { case 1: function h() {} } return typeof h",
        "function",
    ),
    (
        "try { function h() {} } catch (e) {} return typeof h",
        "function",
    ),
    (
        "try { throw 1 } catch (e) { function h() {} } return typeof h",
        "function",
    ),
    (
        "for (var i = 0; i < 1; i++) { function h() {} } return typeof h",
        "function",
    ),
    (
        "while (true) { function h() {} break } return typeof h",
        "function",
    ),
    ("var t = typeof h; { function h() {} } return t", "undefined"),
    (
        "{ function h() { return 1 } } { function h() { return 2 } } return h()",
        "2",
    ),
    (
        "'use strict'; { function h() {} } return typeof h",
        "undefined",
    ),
    (
        "'use strict'; { function h() { return 3 } return h() }",
        "3",
    ),
    (
        "'use strict'; switch (1) { case 1: function h() { return 2 } return h() }",
        "2",
    ),
    (
        "'use strict'; try { throw 1 } catch (e) { function h() { return 1 } return h() }",
        "1",
    ),
    (
        "var r = []; for (var i = 0; i < 2; i++) { function h() { return i } r.push(h) } return r[0] === r[1]",
        "false",
    ),
];

#[test]
fn an_annex_b_block_function_is_visible_as_node_shows_it_or_bails_in_both_modes() {
    answers_as_node_or_bails(ANNEX_B_BLOCK_FUNCTIONS, false);
}

const ANNEX_B_AT_THE_TOP_LEVEL: &[(&str, &str, &str)] = &[
    ("{ function h() {} }", "typeof h", "function"),
    ("if (true) { function h() {} }", "typeof h", "function"),
    ("var t = typeof h; { function h() {} }", "t", "undefined"),
    (
        "switch (1) { case 1: function h() { return 3 } }",
        "h()",
        "3",
    ),
];

#[test]
fn an_annex_b_block_function_at_the_top_level_answers_as_node_or_bails_in_both_modes() {
    let mut wrong = Vec::new();
    for (prelude, expr, node) in ANNEX_B_AT_THE_TOP_LEVEL {
        if let Some(printed) = printed_by_node(&format!("{prelude} console.log(String({expr}));")) {
            assert_eq!(printed.trim_end(), *node, "node on {prelude}");
        }
        for decoding in [false, true] {
            let records = units(&format!("{prelude} String({expr});"), decoding);
            let fine = records.iter().all(|unit| unit.bail.is_some())
                || records
                    .last()
                    .is_some_and(|unit| unit.values.last() == Some(&format!("string:{node}")));
            if !fine {
                wrong.push(format!("decoding={decoding} {records:?}: {prelude}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

const SLOPPY_WRITES_TO_A_PRIMITIVE: &[(&str, &str)] = &[
    ("var s = 'x'; s.p = 1; return typeof s.p", "undefined"),
    ("var s = 'x'; s.p = 1; return 'ok'", "ok"),
    ("var s = 'x'; return s.p = 1", "1"),
    ("var n = 5; n.p = 1; return typeof n.p", "undefined"),
    ("var b = true; b.p = 1; return 'ok'", "ok"),
    ("var s = 'x'; s[0] = 'y'; return s", "x"),
    ("var s = 'x'; s.length = 4; return s.length", "1"),
    ("var s = 'x'; s.p += 1; return typeof s.p", "undefined"),
    ("var s = 'x'; s.p++; return typeof s.p", "undefined"),
    (
        "var n = 1; try { null.p = 1; return 'no' } catch (e) { return e.name }",
        "TypeError",
    ),
    (
        "'use strict'; var s = 'x'; try { s.p = 1; return 'no' } catch (e) { return e.name }",
        "TypeError",
    ),
    (
        "var s = 'x'; return (function () { 'use strict'; try { s.p = 1; return 'no' } catch (e) { return e.name } })()",
        "TypeError",
    ),
];

#[test]
fn a_sloppy_write_to_a_primitive_is_ignored_as_node_does_or_bails_in_both_modes() {
    answers_as_node_or_bails(SLOPPY_WRITES_TO_A_PRIMITIVE, false);
}
