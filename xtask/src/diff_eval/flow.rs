use super::pool::Rng;
use super::statements::{pick_key, pick_target, pick_value};

const FOR_IN_TARGETS: &[&str] = &[
    "{ a: 1, b: 2, c: 3 }",
    "{ 2: 'x', a: 1, 1: 'y', b: 2 }",
    "[1, 2, 3, 4]",
    "['a', 'b']",
    "Object.create({ k: 1, toString: 2 })",
    "'str'",
    "(function () { return arguments; })(1, 2, 3)",
    "/a/g",
    "function f() {}",
];

const FOR_IN_MUTATIONS: &[&str] = &[
    "delete o[@K]",
    "o[@K] = 9",
    "o.length = 1",
    "o.pop && o.pop()",
    "o.shift && o.shift()",
    "o.splice && o.splice(0, 1)",
    "delete o.a",
    "o.z = 1",
];

const ITERABLES: &[&str] = &[
    "[1, 2, 3]",
    "'ab\\ud83d\\ude00'",
    "(function () { return arguments; })(4, 5)",
    "[]",
    "{ length: 1, 0: 'x' }",
    "5",
    "null",
    "new (function () {})()",
];

const LOOPS: &[&str] = &[
    "var fs = []; for (let i = 0; i < 3; i++) { fs.push(function () { return i; }); } return [fs.map(function (f) { return f(); }), log];",
    "var fs = []; for (let i = 0; i < 3; i++) { fs.push(function () { return i; }); i += 1; } return [fs.map(function (f) { return f(); })];",
    "var fs = []; for (let i = 0, n = (log += 'n', 2); i < n; i++) { fs.push(() => i * n); } return [fs.map(function (f) { return f(); }), log];",
    "var r = []; outer: for (var i = 0; i < 3; i++) { for (var j = 0; j < 3; j++) { if (j === @A) continue outer; if (i === @B) break outer; r.push(i * 10 + j); } } return [r];",
    "var i = 0; do { log += i; if (i === @A) break; } while (++i < 4); return [i, log];",
    "var i = 0; while (true) { if (i++ > @A) break; if (i === @B) continue; log += i; } return [i, log];",
    "for (var i = (log += 'i', 0); log += 't', i < 2; log += 'u', i++) { log += 'b'; } return [log, i];",
    "switch (@V) { case 1: log += '1'; case '1': log += 's'; break; default: log += 'd'; case 2: log += '2'; } return [log];",
    "try { x; let x = 1; } catch (e) { return ['tdz', e instanceof ReferenceError]; } return ['no'];",
    "try { for (let x of (x, [1])) {} } catch (e) { return ['tdz']; } return ['no'];",
    "try { for (let i = (i, 0); i < 1; i++) {} } catch (e) { return ['tdz']; } return ['no'];",
    "var x = 'outer'; try { for (const x of (log += typeof x, [1])) {} } catch (e) { return ['tdz', log]; } return ['no', log];",
    "var s = 0; for (var k in [5, 6]) { s += k; } return [s];",
    "label: { log += 'a'; if (@A) break label; log += 'b'; } return [log];",
];

const BLOCK_FUNCTIONS: &[&str] = &[
    "var x = @V; { function x() { return 1; } } return [typeof x];",
    "{ function g() { return 1; } } return [typeof g];",
    "var r = typeof g; { function g() {} } return [r, typeof g];",
    "if (@A) function g() { return 2; } return [typeof g];",
    "if (@A) ; else function g() {} return [typeof g];",
    "switch (1) { case 1: function g() {} } return [typeof g];",
    "l: function g() {} return [typeof g];",
    "{ let g = 1; { function g() {} } return [typeof g]; }",
    "function outer() { var r = typeof g; { function g() {} } return [r, typeof g]; } return outer();",
    "try { throw 1; } catch (g) { { function g() {} } } return [typeof g];",
    "{ function g() { return 1; } { function g() { return 2; } } } return [g()];",
    "var g = 5; (function () { { function g() {} } })(); return [typeof g];",
    "for (var i = 0; i < 1; i++) { function g() { return i; } } return [typeof g];",
    "{ function g() {} g = 3; } return [typeof g];",
];

const SAMPLES: &[&str] = &[
    "return ['kiss_me Sky!'.replace(/[\\W_]+/gi, '')];",
    "return [/[\\W]/i.test('s'), /[\\W]/i.test('k'), /[\\W]/i.test('K'), /\\W/i.test('s')];",
    "return ['ask'.replace(/[^\\W]/gi, '.')];",
    "return ['ask_K'.replace(/[^\\W_]/gui, '.')];",
    "return ['ask'.replace(/[^\\W]/gvi, '.')];",
    "return [/k/ui.test('K') + ',' + /[^\\W]/ui.test('K')];",
    "return [RegExp('(?i:a)b').test('Ab')];",
    "return [/\\p{L}/u.test('\\ua7ce')];",
    "return [/\\ua7ce/ui.test('\\ua7cf')];",
    "return [/\\p{Cn}/u.test('\\ua7ce')];",
    "return [RegExp('\\\\p{Script=Beria_Erfe}', 'u').test('a')];",
    "return ['A\\u03a3\\u1acfB'.toLowerCase().charCodeAt(1), 'A\\u1acf\\u03a3'.toLowerCase().charCodeAt(2)];",
    "return ['A\\u03a3\\u0295B'.toLowerCase().charCodeAt(1)];",
    "return ['A\\u03a3\\ud806\\udf60B'.toLowerCase().charCodeAt(1)];",
    "return [Object.prototype.toString.call(function* () {}.bind(null)), Object.prototype.toString.call(async function () {}.bind(null))];",
    "var c = 0; try { ({ toString: function () { c++; return 'a'; } }) in 'str'; } catch (z) {} return [c];",
    "var c = 0; try { null[{ toString: function () { c++; return 'a'; } }]; } catch (z) {} return [c];",
    "var o = {}; o[{ toString: function () { log += 'k'; return 'p'; } }] = (log += 'v', 1); return [log];",
    "var o = { p: 1 }; o[{ toString: function () { log += 'k'; return 'p'; } }] += (log += 'v', 1); return [log, o.p];",
    "var o = { p: 1 }; o[{ toString: function () { log += 'k'; return 'p'; } }]++; return [log, o.p];",
    "var o = { p: 0 }; o[{ toString: function () { log += 'k'; return 'p'; } }] ||= (log += 'v', 2); return [log, o.p];",
    "var f = function (a, b) {}; f.length = 9; return [f.length];",
    "var f = function g() {}; f.name = 'z'; return [f.name];",
    "var f = () => 1; f.name = 'z'; return [f.name];",
    "var x = 1; { function x() {} } return [typeof x];",
    "return [/[\\W_]+/gi, /[^\\W]/ui, '(?i:a)b', /\\p{L}/u];",
];

fn one<'s>(rng: &mut Rng, list: &[&'s str]) -> &'s str {
    list[rng.below(list.len())]
}

fn small(rng: &mut Rng) -> String {
    rng.below(4).to_string()
}

fn fill(rng: &mut Rng, template: &str) -> String {
    template
        .replace("@K", pick_key(rng))
        .replace("@A", &small(rng))
        .replace("@B", &small(rng))
        .replace("@V", pick_value(rng))
}

pub fn loop_case(rng: &mut Rng) -> (String, String) {
    match rng.below(4) {
        0 => for_in(rng),
        1 => for_of(rng),
        _ => {
            let template = one(rng, LOOPS);
            (
                format!("stmt loop {}", &template[..template.len().min(24)]),
                fill(rng, template),
            )
        }
    }
}

fn for_in(rng: &mut Rng) -> (String, String) {
    let target = if rng.chance(70) {
        one(rng, FOR_IN_TARGETS)
    } else {
        pick_target(rng)
    };
    let mutation = one(rng, FOR_IN_MUTATIONS).replace("@K", pick_key(rng));
    let at = small(rng);
    let body = format!(
        "var o = {target}; var ks = []; var i = 0; for (var k in o) {{ ks.push(k); if (i++ === {at}) {{ {mutation}; }} }} return [ks, o, log];"
    );
    ("stmt for-in".to_string(), body)
}

fn for_of(rng: &mut Rng) -> (String, String) {
    let target = one(rng, ITERABLES);
    let at = small(rng);
    let body = format!(
        "var o = {target}; var r = []; for (var x of o) {{ r.push(x); if (r.length === {at}) {{ if (o.push) o.push('m'); else break; }} if (r.length > 6) break; }} return [r, log];"
    );
    ("stmt for-of".to_string(), body)
}

pub fn block_function_case(rng: &mut Rng) -> (String, String) {
    let template = one(rng, BLOCK_FUNCTIONS);
    (
        format!("stmt block fn {}", &template[..template.len().min(28)]),
        fill(rng, template),
    )
}

pub fn sample_case(rng: &mut Rng) -> (String, String) {
    let index = rng.below(SAMPLES.len());
    (format!("stmt sample {index}"), SAMPLES[index].to_string())
}
