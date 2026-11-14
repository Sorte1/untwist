use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::builtins::{intact, scan, Builtins, Reliance};
use untwist_core::semantic::{build_scoping, has_module_syntax};

const MATH_ROUND: Reliance = Reliance {
    roots: &["Math"],
    members: &[("Math", false, &["round"])],
};

fn verdict(source: &str) -> Builtins {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    scan(&parsed.program, &sem.scoping, module, false, &MATH_ROUND).verdict
}

fn first_look_is_intact(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    intact(&parsed.program, &sem.scoping, module, &MATH_ROUND)
}

#[test]
fn a_program_that_only_calls_the_member_is_intact() {
    assert_eq!(
        verdict("x = Math.round(1.5) + Math.floor(2.5);"),
        Builtins::Intact
    );
    assert!(first_look_is_intact("x = Math.round(1.5);"));
}

#[test]
fn a_write_to_a_member_nobody_relies_on_is_intact() {
    assert_eq!(verdict("Math.floor = function () {};"), Builtins::Intact);
}

#[test]
fn a_write_to_a_relied_on_member_is_written() {
    for source in [
        "Math.round = function () {};",
        "Math['round'] = function () {};",
        "Math[key] = function () {};",
        "delete Math.round;",
        "Object.defineProperty(Math, 'round', {});",
        "Reflect.set(Math, 'round', f);",
        "Object.assign(Math, patch);",
        "Math = {};",
        "globalThis.Math = {};",
        "window.Math.round = f;",
        "this.Math = {};",
        "globalThis[name] = {};",
    ] {
        assert_eq!(verdict(source), Builtins::Written, "{source}");
    }
}

#[test]
fn a_root_used_as_a_value_is_aliased() {
    for source in [
        "var M = Math;",
        "f(Math);",
        "var g = globalThis;",
        "f(this);",
    ] {
        assert_eq!(verdict(source), Builtins::Aliased, "{source}");
        assert!(!first_look_is_intact(source), "{source}");
    }
}

#[test]
fn a_root_outside_the_reliance_is_not_watched() {
    assert_eq!(verdict("var O = Object; JSON.parse = f;"), Builtins::Intact);
}

#[test]
fn a_local_binding_of_the_same_name_is_not_the_built_in() {
    assert_eq!(
        verdict("function f(Math) { Math.round = 1; return Math; }"),
        Builtins::Intact
    );
}

const ARRAY_CONCAT: Reliance = Reliance {
    roots: &["Array"],
    members: &[
        ("Array", false, &["prototype"]),
        ("Array", true, &["concat"]),
    ],
};

const JSON_PARSE: Reliance = Reliance {
    roots: &["JSON"],
    members: &[("JSON", false, &["parse"])],
};

fn verdict_for(source: &str, reliance: &Reliance) -> Builtins {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    scan(&parsed.program, &sem.scoping, module, false, reliance).verdict
}

#[test]
fn a_prototype_reached_through_a_value_and_written_is_not_intact() {
    for source in [
        "[].__proto__.concat = f;",
        "[]['__proto__'].concat = f;",
        "[][`__proto__`].concat = f;",
        "[].constructor.prototype.concat = f;",
        "[].constructor['prototype'].concat = f;",
        "Object.getPrototypeOf([]).concat = f;",
        "Reflect.getPrototypeOf([]).concat = f;",
        "Reflect.get([], '__proto__').concat = f;",
        "Object.getPrototypeOf.call(null, []).concat = f;",
        "[].__proto__['concat'] = f;",
        "[].__proto__[key] = f;",
        "delete [].__proto__.concat;",
        "[].__proto__.concat += f;",
        "[].__proto__.concat.call = f;",
        "Object.defineProperty([].__proto__, 'concat', { value: f });",
        "Object.assign(Object.getPrototypeOf([]), { concat: f });",
        "Object.setPrototypeOf([].__proto__, p);",
        "[].__proto__.__defineGetter__('concat', f);",
        "function g(a) { a.__proto__.concat = f; }",
        "if (!Array.prototype.concat) { Array.prototype.concat = 1; } [].__proto__['concat'] = f;",
        "x.Array = f;",
    ] {
        assert_eq!(
            verdict_for(source, &ARRAY_CONCAT),
            Builtins::Aliased,
            "{source}"
        );
    }
}

#[test]
fn a_prototype_reached_through_a_value_that_escapes_is_aliased() {
    for source in [
        "(function (A) { A.prototype.concat = f; })([].constructor);",
        "var p = [].__proto__;",
        "var p = x.__proto__ || y;",
        "var g = Object.getPrototypeOf;",
        "var g = Object.getPrototypeOf.bind();",
        "var p = x?.__proto__;",
        "function g() { return x.constructor; }",
        "var o = { p: [].constructor.prototype };",
        "var c = x.__lookupGetter__('__proto__').call([]);",
        "var c = x.constructor('return this')();",
        "var c = new x.constructor('return this');",
        "Function('return this')();",
        "new Function('Array.prototype.concat = f')();",
        "(0, eval)('Array.prototype.concat = f');",
        "eval('Array.prototype.concat = f');",
        "setTimeout('Array.prototype.concat = f');",
        "window.setTimeout(code, 0);",
        "require('vm').runInThisContext('Array.prototype.concat = f');",
        "var A = top.Array;",
        "(0, x.constructor)('return this');",
        "(a || Function)('return this');",
    ] {
        assert_eq!(
            verdict_for(source, &ARRAY_CONCAT),
            Builtins::Aliased,
            "{source}"
        );
    }
}

#[test]
fn reading_through_a_reached_prototype_without_letting_it_go_is_intact() {
    for source in [
        "if (x.constructor === Array) {}",
        "y = x.constructor.name;",
        "y = typeof x.constructor;",
        "y = x.constructor && x.constructor.name;",
        "y = Object.prototype.hasOwnProperty.call(o, k);",
        "y = x.__proto__.toString.call(o);",
        "y = o instanceof x.constructor;",
        "y = Object.getPrototypeOf(x) === Array.prototype;",
        "setTimeout(function () {}, 1); setTimeout(() => 1); function tick() {} setInterval(tick, 5); setTimeout(o.f.bind(o));",
        "x.constructor.count = 1;",
        "\"Object\" === t && r.constructor && (t = r.constructor.name);",
        "y = (0, x.constructor, 1);",
        "if (\"Object\" === t && r.constructor) t = r.constructor.name;",
        "while (!x.constructor) {}",
        "y = x.__proto__ ? 1 : 2;",
    ] {
        assert_eq!(verdict_for(source, &JSON_PARSE), Builtins::Intact, "{source}");
    }
    for source in [
        "if (x.constructor === Array) {}",
        "y = x.constructor.name;",
        "y = x.__proto__.toString.call(o);",
        "y = Object.getPrototypeOf(x) === Array.prototype;",
    ] {
        assert_eq!(
            verdict_for(source, &ARRAY_CONCAT),
            Builtins::Intact,
            "{source}"
        );
    }
}

#[test]
fn json_reached_through_a_value_and_written_is_not_intact() {
    for source in [
        "(function () { return this; })().JSON.parse = f;",
        "Function('return this')().JSON.parse = f;",
        "document.defaultView.JSON.parse = f;",
        "top.JSON.parse = f;",
        "globalThis[k].parse = f;",
        "x.constructor.constructor('return this')().JSON = { parse: f };",
    ] {
        assert_ne!(
            verdict_for(source, &JSON_PARSE),
            Builtins::Intact,
            "{source}"
        );
    }
}

#[test]
fn a_prototype_read_through_a_pattern_or_a_spelled_key_is_aliased() {
    for source in [
        "var { __proto__: p } = []; p.round = 1;",
        "var { constructor: c } = [];",
        "var { __proto__ } = [];",
        "({ __proto__: p } = []);",
        "({ __proto__ } = []);",
        "function f({ __proto__: p }) { return p; }",
        "var k = '__proto__'; var { [k]: p } = [];",
        "var p = []['__pro' + 'to__'];",
        "var p = []['__pro' + `to__`];",
        "var p = [][`__${k}__`];",
        "var k = '__pro'; k += 'to__'; var p = [][k];",
        "var k = '__proto__'; x = [][k];",
        "var a = '__pro', b = 'to__'; x = [][a + b];",
        "var a = '__pro', b = 'to__', k = a + b; x = [][k];",
        "var o = { k: '__proto__' }; x = [][o.k];",
        "function g(o, k) { return o[k]; } x = g([], '__proto__');",
        "function g(o, k) { return o[k]; } x = g([], `constructor`);",
        "var parts = 'a constructor'.split(' '); x = (function () {})[parts[1]];",
        "var [k] = ['__proto__']; x = [][k];",
        "var k = 'a'; k = 'con' + 'structor'; x = (function () {})[k];",
        "var k; ({ k = '__proto__' } = {}); x = [][k];",
        "for (var k of ['__proto__']) { x = [][k]; }",
        "var j = '__proto__', k = j; x = [][k];",
        "x = [][atob('X19wcm90b19f')];",
        "x = [][String.fromCharCode(95, 95, 112, 114, 111, 116, 111, 95, 95)];",
    ] {
        assert_eq!(verdict(source), Builtins::Aliased, "{source}");
    }
}

#[test]
fn a_key_that_cannot_spell_a_prototype_name_stays_intact() {
    for source in [
        "var a = [1, 2]; for (var i = 0; i < a.length; i++) { x = a[i]; }",
        "var a = [1, 2], n = 0; n += 1; x = a[n * 2] + a[n - 1] + a[-n];",
        "var a = [1, 2], j = 0, k = j; x = a[k];",
        "var o = { ab: 1 }; x = o['a' + 'b'];",
        "x = [][[]];",
        "x = o[typeof y] + o[!y];",
        "var o = { foo: 1 }; x = o[atob('Zm9v')];",
        "var k = 'dyn'; x = o[k];",
        "for (var i = 0; i < n; i++) { x = a[i + 1] + a[1 + i]; }",
        "x = r[Symbol.iterator] || r['@@iterator'];",
        "var { a: b } = o;",
        "({ a } = o);",
    ] {
        assert_eq!(verdict(source), Builtins::Intact, "{source}");
    }
}

#[test]
fn a_key_from_a_parameter_an_enumeration_or_an_opaque_call_aliases_the_built_ins() {
    for source in [
        "function f(k) { return [][k]; }",
        "for (var k in o) { x = o[k]; }",
        "x = cache[id()];",
        "for (var p in b) { d[p] = b[p]; }",
        "function f(o, k) { var v = o[k]; return v; }",
    ] {
        assert_eq!(verdict(source), Builtins::Aliased, "{source}");
        assert!(!first_look_is_intact(source), "{source}");
    }
}
