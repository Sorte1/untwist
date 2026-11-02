mod common;

use oxc_allocator::Allocator;

const HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
";

fn decoder(test: &str) -> String {
    format!(
        "function __DECODE_0__(c) {{ var e = __STRING_ARRAY__(); return e[c] + (({test}) ? '!' : '?'); }}\n"
    )
}

fn run(test: &str, patch: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = format!(
        "{HEADER}{}{patch}\nconsole.log(__DECODE_0__(0x0));\n",
        decoder(test)
    );
    let source = alloc.alloc_str(&source);
    common::run_ctors(&alloc, source, &[untwist_deobfuscate::decoded::boxed])
}

fn refused(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note.starts_with(untwist_deobfuscate::notes::BUILTIN_PATCHED))
}

const TESTS: [&str; 4] = [
    "[] instanceof Object",
    "new Object() instanceof Object",
    "/x/ instanceof Object",
    "(function () {}) instanceof Object",
];

const KEEPING_WRITES: [&str; 39] = [
    "var d = Object.getOwnPropertyDescriptor({}, 'x');",
    "",
    "var o = {}; o.x = 1;",
    "var arr = []; arr[0] = 'x'; delete arr[0];",
    "function F() {} F.y = Object.keys({});",
    "var t = {}; Object.assign(t, { k: 1 });",
    "var { a: q, b } = { a: 1, b: 2 }; var [m] = [q + b];",
    "var r = [1, 2]; var s = 0; for (var i = 0; i < r.length; i = i + 1) { s += r[i] * 2; }",
    "var r = [1, 2]; var has = 0 in r;",
    "var o = { constructor: 1, get g() { return 2; }, set s(v) {}, m() {} }; var v = o.g;",
    "class K { static ['m' + 'x']() { return 1; } } var kv = K.mx();",
    "var tag = function (s) { return s[0]; }; var tv = tag`x`;",
    "var oc = { a: { b: 1 } }; var ov = oc?.a?.b;",
    "Math.max = function () { return 0; };",
    "Array.prototype.flatMap = function () { return []; };",
    "function g(p) { p.x = 1; } g({});",
    "var t = (function () { return this; })(); t.x = 1;",
    "x = {};",
    "var o = {}; o = Array.prototype; o.x = 1;",
    "var o = {}; var k = 'assign'; o[k](Array.prototype, {});",
    "Object.__defineGetter__('x', function () {});",
    "var g = Function('return this')(); g.x = 1;",
    "for (var o of [{}]) o.x = 1;",
    "var o = {}; for (o in {}) o.x = 1;",
    "require('fs');",
    "var f = [].slice; f.call({});",
    "var k = 'constr' + 'uctor'; var F = [][k];",
    "var F = [][`constructor`];",
    "function h(n) { return [][n]; } h(0);",
    "var { constructor: C } = [];",
    "var { constructor } = [];",
    "var c; ({ constructor: c } = []);",
    "(function ({ constructor: { constructor: F } }) { return F; })(0);",
    "var [{ constructor: C }] = [[]];",
    "var c; [{ constructor: c }] = [[]];",
    "function f(...{ constructor: C }) { return C; } f([]);",
    "var has = 'constructor' in [];",
    "var k = 'constr' + 'uctor'; var has = k in [];",
    "var o = { __proto__: [] };",
];

const REFUSING_WRITES: [&str; 24] = [
    "Function('Object = function () {}')();",
    "Function`Object = function () {}`();",
    "var k = 'constructor'; var F = [][k][k]; F`Object = function () {}`();",
    "Function('function h() { Object = function () {}; } h();')();",
    "Object = function () {};",
    "Object.defineProperty(Object, Symbol.hasInstance, { value: function () { return false; } });",
    "Object[Symbol.hasInstance] = function () { return false; };",
    "Object.prototype.x = 1;",
    "Object['prototype'] = {};",
    "Object.setPrototypeOf(Object, null);",
    "window.Object = function () {};",
    "var O = Object; O.x = 1;",
    "function keep(f) { return f; } keep(Object);",
    "Array.prototype.__proto__ = null;",
    "Object.assign(Array.prototype, {});",
    "var d = Object.getOwnPropertyDescriptor(Object, 'defineProperty');",
    "var k = 'x'; for (var i = 0; i < 2; i++) { k = k + i; } var v = [][k];",
    "var oc = []; var C = oc?.constructor;",
    "class A extends Array { m() { return super.constructor; } } var F = new A().m();",
    "Reflect.setPrototypeOf(Array.prototype, null);",
    "var g = globalThis; g['Obj' + 'ect'] = function () {};",
    "var s = Object.setPrototypeOf; s(Array.prototype, null);",
    "var s = ({}).__lookupSetter__('__proto__'); s.call(Array.prototype, null);",
    "var { setPrototypeOf } = Object; setPrototypeOf(Array.prototype, null);",
];

#[test]
fn a_bare_object_operand_of_instanceof_decodes_when_no_write_reaches_object() {
    let mut wrong = Vec::new();
    for test in TESTS {
        for patch in KEEPING_WRITES {
            let (code, notes) = run(test, patch);
            if refused(&notes) || !code.contains("console.log(\"log!\")") {
                wrong.push(format!("{test} / {patch}: {notes:?}\n{code}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_write_that_reaches_object_keeps_the_bare_operand_a_patched_root() {
    let mut wrong = Vec::new();
    for test in TESTS {
        for patch in REFUSING_WRITES {
            let (code, notes) = run(test, patch);
            if !refused(&notes) || !code.contains("__DECODE_0__(0)") {
                wrong.push(format!("{test} / {patch}: {notes:?}\n{code}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
