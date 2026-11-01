use untwist_eval::builtins::shape::{
    Kinds, ARGUMENTS, ARRAY, BOOL, CALLABLE, ERROR, NULL, NUMBER, PLAIN, REGEXP, STRING, UNDEFINED,
};

pub struct Sample {
    pub source: &'static str,
    pub kind: Kinds,
    pub foldable: bool,
}

const fn sample(source: &'static str, kind: Kinds, foldable: bool) -> Sample {
    Sample {
        source,
        kind,
        foldable,
    }
}

pub static POOL: &[Sample] = &[
    sample("undefined", UNDEFINED, true),
    sample("null", NULL, true),
    sample("true", BOOL, true),
    sample("false", BOOL, true),
    sample("0", NUMBER, true),
    sample("-0", NUMBER, true),
    sample("1", NUMBER, true),
    sample("-1", NUMBER, true),
    sample("2", NUMBER, true),
    sample("3", NUMBER, true),
    sample("10", NUMBER, true),
    sample("16", NUMBER, true),
    sample("36", NUMBER, true),
    sample("37", NUMBER, true),
    sample("1.5", NUMBER, true),
    sample("-1.5", NUMBER, true),
    sample("2.5", NUMBER, true),
    sample("0.1", NUMBER, true),
    sample("1e21", NUMBER, true),
    sample("1e-7", NUMBER, true),
    sample("5e-324", NUMBER, true),
    sample("255", NUMBER, true),
    sample("65536", NUMBER, true),
    sample("4294967296", NUMBER, true),
    sample("-2147483649", NUMBER, true),
    sample("9007199254740993", NUMBER, true),
    sample("NaN", NUMBER, true),
    sample("Infinity", NUMBER, true),
    sample("-Infinity", NUMBER, true),
    sample("''", STRING, true),
    sample("'a'", STRING, true),
    sample("'abc'", STRING, true),
    sample("'aBc'", STRING, true),
    sample("'ABC'", STRING, true),
    sample("' a b '", STRING, true),
    sample("'0'", STRING, true),
    sample("'1'", STRING, true),
    sample("'-1'", STRING, true),
    sample("'1.5'", STRING, true),
    sample("'0x1f'", STRING, true),
    sample("'0b11'", STRING, true),
    sample("'1e3'", STRING, true),
    sample("'  12px'", STRING, true),
    sample("'Infinity'", STRING, true),
    sample("'a,b,c'", STRING, true),
    sample("'a.b*c'", STRING, true),
    sample("'%41%e2%82%ac'", STRING, true),
    sample("'%E0%A4%A'", STRING, true),
    sample("'YWJj'", STRING, true),
    sample("'YQ=='", STRING, true),
    sample("'$&-$1-$`-$\\''", STRING, true),
    sample("'\\u017f\\u00df\\u0130'", STRING, true),
    sample("'\\u00e9t\\u00e9'", STRING, true),
    sample("'\\u03a3A\\u03a3'", STRING, true),
    sample("'\\ud800x'", STRING, true),
    sample("'\\ud83d\\ude00'", STRING, true),
    sample("'\\n\\t\"\\\\'", STRING, true),
    sample(
        "'12345678901234567890123456789012345678901234567890'",
        STRING,
        true,
    ),
    sample("'ffffffffffffffffffffffffffffffffffffffffff'", STRING, true),
    sample("'__proto__'", STRING, true),
    sample("'length'", STRING, true),
    sample("'{\"b\":1,\"2\":[1,\"x\"],\"a\":null}'", STRING, true),
    sample("'[1,2,3]'", STRING, true),
    sample("'g'", STRING, true),
    sample("'gi'", STRING, true),
    sample("'y'", STRING, true),
    sample("'uv'", STRING, true),
    sample("'vu'", STRING, true),
    sample("'iu'", STRING, true),
    sample("'sv'", STRING, true),
    sample("[]", ARRAY, true),
    sample("[1, 2, 3]", ARRAY, true),
    sample("['a', 'b', 'a']", ARRAY, true),
    sample("[3, 1, 2, 1]", ARRAY, true),
    sample("[1, [2, 3], 'x']", ARRAY, true),
    sample("[null, undefined, 0]", ARRAY, true),
    sample("[-0, NaN]", ARRAY, true),
    sample("['\\u00df', 'b']", ARRAY, true),
    sample("[1, , 3]", ARRAY, false),
    sample("{}", PLAIN, true),
    sample("{ a: 1, b: 'x' }", PLAIN, true),
    sample("{ b: 1, 2: 1, a: 1, 1: 1 }", PLAIN, true),
    sample("{ length: 2, 0: 'p', 1: 'q' }", PLAIN, true),
    sample("{ valueOf: function () { return 7; } }", PLAIN, false),
    sample("{ toString: function () { return 'ts'; } }", PLAIN, false),
    sample(
        "{ valueOf: function () { return {}; }, toString: function () { return '9'; } }",
        PLAIN,
        false,
    ),
    sample("{ toString: function () { return {}; } }", PLAIN, false),
    sample(
        "{ value: 4, writable: true, enumerable: true, configurable: true }",
        PLAIN,
        true,
    ),
    sample("function (a) { return a; }", CALLABLE, false),
    sample("function (a, b) { return a + b; }", CALLABLE, false),
    sample("function (v, k) { return k; }", CALLABLE, false),
    sample("function (v, k, o) { o.pop(); return v; }", CALLABLE, false),
    sample(
        "function (v, k, o) { o[2] = 'w'; return v > 1; }",
        CALLABLE,
        false,
    ),
    sample("function () { return arguments.length; }", CALLABLE, false),
    sample(
        "function (m, p) { return '[' + m + ':' + p + ']'; }",
        CALLABLE,
        false,
    ),
    sample("function () { return this; }", CALLABLE, false),
    sample("function f(a) { return typeof f + a; }", CALLABLE, false),
    sample("() => 'x'", CALLABLE, false),
    sample("parseInt", CALLABLE, false),
    sample("String", CALLABLE, false),
    sample("Number", CALLABLE, false),
    sample("Boolean", CALLABLE, false),
    sample("/a/", REGEXP, false),
    sample("/a/g", REGEXP, false),
    sample("/[a-z]/gi", REGEXP, false),
    sample("/(a)(b)?/", REGEXP, false),
    sample("/x*/g", REGEXP, false),
    sample("/\\d+/g", REGEXP, false),
    sample("/b/y", REGEXP, false),
    sample("/^./m", REGEXP, false),
    sample("/a|/g", REGEXP, false),
    sample("/,/", REGEXP, false),
    sample("/(?:)/", REGEXP, false),
    sample("/\\u00df/i", REGEXP, false),
    sample("1.005", NUMBER, true),
    sample("0.5", NUMBER, true),
    sample("123.456", NUMBER, true),
    sample("-1e21", NUMBER, true),
    sample("9007199254740992", NUMBER, true),
    sample("0.000001", NUMBER, true),
    sample("1e400", NUMBER, true),
    sample("'\\u00a0 1 \\ufeff'", STRING, true),
    sample("'a\\u2028b'", STRING, true),
    sample("'\\u0130\\ufb00\\u0149'", STRING, true),
    sample("'%uD800x%u0041%4'", STRING, true),
    sample(r#"'"\\u00e9"'"#, STRING, true),
    sample("'{a:1}'", STRING, true),
    sample("' -0 '", STRING, true),
    sample("'1e400'", STRING, true),
    sample(r#"'[{"x":[1,{"y":"\\n"}]},true,null]'"#, STRING, true),
    sample("'.5e-3'", STRING, true),
    sample("'0o17'", STRING, true),
    sample("'-0x10'", STRING, true),
    sample("[{ a: 1 }, [], 'z']", ARRAY, true),
    sample("['10', '9', '1']", ARRAY, true),
    sample("/(a)\\1/", REGEXP, false),
    sample("/(?<=a)b/g", REGEXP, false),
    sample("/[^]/", REGEXP, false),
    sample("/\\bB/i", REGEXP, false),
    sample("/\\W/i", REGEXP, false),
    sample("/./s", REGEXP, false),
    sample("/^\\s+|\\s+$/g", REGEXP, false),
    sample("/(a)|(b)/g", REGEXP, false),
    sample("/[\\u00e0-\\u00ff]/u", REGEXP, false),
    sample(
        "(function () { return arguments; })(1, 'two', 3)",
        ARGUMENTS,
        false,
    ),
    sample("(function () { return arguments; })()", ARGUMENTS, false),
    sample(
        "{ valueOf: function () { r[0] = 'm'; return 0; } }",
        PLAIN,
        false,
    ),
    sample(
        "{ valueOf: function () { r.length = 0; return 1; } }",
        PLAIN,
        false,
    ),
    sample(
        "{ valueOf: function () { r.push(9); return 1; } }",
        PLAIN,
        false,
    ),
    sample(
        "{ toString: function () { r.pop(); return ','; } }",
        PLAIN,
        false,
    ),
    sample(
        "{ toString: function () { r[1] = 'w'; return 'b'; } }",
        PLAIN,
        false,
    ),
    sample(
        "[{ toString: function () { r[2] = 'z'; return 'e'; } }, 'f']",
        ARRAY,
        false,
    ),
    sample("function* () { yield 1; }", CALLABLE, false),
    sample("async function () { return 1; }", CALLABLE, false),
    sample("async () => 1", CALLABLE, false),
    sample("({ m() { return 'm'; } }).m", CALLABLE, false),
    sample(
        "(function (a, b) { return a + b; }).bind(null, 1)",
        CALLABLE,
        false,
    ),
    sample("String.prototype.toUpperCase", CALLABLE, false),
    sample("Object.create(function named(a, b) {})", PLAIN, false),
    sample("Object.create(/a/g)", PLAIN, false),
    sample("Object.create([1, 2])", PLAIN, false),
    sample(
        "Object.create(Object.create(function f() {}))",
        PLAIN,
        false,
    ),
    sample("Object.create({ k: 1 })", PLAIN, false),
    sample(
        "(function () { try { null.x; } catch (e) { return e; } })()",
        ERROR,
        false,
    ),
    sample("'\\ua7ce\\ua7d2\\ua7d5'", STRING, true),
    sample("'\\ud81b\\udea0\\ud81b\\udebb'", STRING, true),
    sample("'\\ud801\\udc28\\ud801\\udc00'", STRING, true),
    sample("'\\u01c5\\ufb03\\u1e9e'", STRING, true),
    sample("'\\u0149\\u1f50\\u0390'", STRING, true),
    sample("'\\u2c2f\\ua7c0\\ua7d0'", STRING, true),
    sample("/[^\\W]/ui", REGEXP, false),
    sample("/[\\W_]+/gi", REGEXP, false),
    sample("'(?i:a)b'", STRING, true),
    sample("/\\p{L}/u", REGEXP, false),
    sample("/\\ua7ce/ui", REGEXP, false),
    sample("'\\\\p{Lu}'", STRING, true),
    sample("'A\\u03a3\\u1acfB'", STRING, true),
    sample("'A\\u1acf\\u03a3'", STRING, true),
    sample("(function* () {}).bind(null)", CALLABLE, false),
    sample("(async function () {}).bind(null)", CALLABLE, false),
    sample("Object.create(Math)", PLAIN, false),
    sample("Math", PLAIN, false),
    sample("JSON", PLAIN, false),
    sample("String.prototype", PLAIN, false),
    sample("((function () {}).bind(null)).bind(null)", CALLABLE, false),
];

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound.max(1) as u64) as usize
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

const LITERAL_NAMES: [&str; 6] = ["undefined", "null", "true", "false", "NaN", "Infinity"];

pub fn is_shared(sample: &Sample) -> bool {
    let source = sample.source;
    source
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_')
        && source.starts_with(|ch: char| ch.is_ascii_alphabetic())
        && !LITERAL_NAMES.contains(&source)
}

pub fn pick(
    rng: &mut Rng,
    accepted: Kinds,
    inside: bool,
    fresh_only: bool,
) -> Option<&'static Sample> {
    let wanted: Vec<&Sample> = POOL
        .iter()
        .filter(|sample| (accepted & sample.kind != 0) == inside)
        .filter(|sample| !(fresh_only && is_shared(sample)))
        .collect();
    if wanted.is_empty() {
        return None;
    }
    Some(wanted[rng.below(wanted.len())])
}
