use untwist_core::pass::Note;
use untwist_deobfuscate::notes;

const HEAD: &str = "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\nfunction __DECODE_0__(a, b) {\n  var e = __STRING_ARRAY__();\n  return e[a];\n}\n";

const WRAPPED_UNIT: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));";

const ROTATED_UNIT: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'Hello World!']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { f = f - 0x0; return e[f]; }; return __DECODE_0__(c, d); }
(function (c, d) {
  var k = __DECODE_0__, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) / 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(__STRING_ARRAY__, 0x6);
console[__DECODE_0__(0x0)](__DECODE_0__(0x1));";

const A_DECODER_THAT_BAILS: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f] + 1n; }; return b(c, d); }
console.log(b(0x0));";

const A_DECODER_THAT_SPENDS_THE_HEAP_BUDGET: &str = "\
function a() { var h = ['x']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var t = e[f]['repeat'](4096)['repeat'](4096), u = t['repeat'](1); return u['length'] > 0 ? e[f] : t; }; return b(c, d); }
console.log(b(0x0));";

const A_MUTATING_DECODER: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { f = f - 0x0; e[0] = 'mutated'; return e[f]; }; return __DECODE_0__(c, d); }
console.log(__DECODE_0__(0x0));";

const A_DECODER_READING_OUTER_STATE: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
var n = 0;
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); n++; return e[c]; }
console.log(__DECODE_0__(0x0));";

const A_BUILT_IN_REPLACED: &str = "\
function __STRING_ARRAY__() { var h = [72, 105]; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return String.fromCharCode(e[c]); }
String.fromCharCode = function () { return 'X'; };
console.log(__DECODE_0__(0x0));";

const A_DECODED_VALUE_SPELLING_THE_DECODER: &str = "\
function a() { var h = ['z']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return String.fromCharCode(e[f].charCodeAt(0) - 24); }; return b(c, d); }
console.log(b(0x0));";

const A_REFUSED_UNITS_ORIGINAL_NAME_IS_A_FREE_REFERENCE: &str = "\
(function () {
  function a() { var h = ['z']; a = function () { return h; }; return a(); }
  function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return String.fromCharCode(e[f].charCodeAt(0) - 24); }; return b(c, d); }
  console.log(b(0x0));
})();
typeof b;";

const TWO_REFUSED_UNITS_WHOSE_ORIGINAL_NAMES_COLLIDE: &str = "\
(function () {
  function a() { var h = ['z']; a = function () { return h; }; return a(); }
  function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return String.fromCharCode(e[f].charCodeAt(0) - 24); }; return b(c, d); }
  console.log(b(0x0));
})();
(function () {
  function a() { var h = ['z']; a = function () { return h; }; return a(); }
  function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return String.fromCharCode(e[f].charCodeAt(0) - 24); }; return b(c, d); }
  console.log(b(0x0));
})();";

const AN_OUT_OF_RANGE_INDEX: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0x1f4));";

const AN_UNDECODABLE_CALL: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(unknownGlobal));";

const A_UNIT_REACHED_THROUGH_THE_GLOBAL_OBJECT: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
this[String.fromCharCode(97)]().reverse();
console.log(b(0));";

const A_BOOTSTRAP_WHOSE_TABLE_IS_WRITTEN: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
!(function (e) {
  function n(t) { var m = { exports: {} }; return e[t].call(m.exports, m, m.exports, n), m.exports; }
  e[0] = function (x, y, z) { console.log(b(0)); };
  n.d = function (t, k, g) { Object.defineProperty(t, k, { value: g }); };
  return n(0);
})([function (x, y, z) { z.d(y, 'v', 1); }]);";

const A_UNIT_WHOSE_GLOBAL_NAMES_ARE_LISTED: &str = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0), Object.keys(this).length);";

const A_GLOBAL_HANDED_TO_A_REQUIRED_PATH: &str = "\
function a() { var h = ['log', 'node:path']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var path = require(b(1));
try { path.basename(this); } catch (error) { console.log(b(0)); }";

const AN_ARRAY_OF_ARRAYS: &str = "\
function __STRING_ARRAY__() { var h = [['x']]; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); return e[c]; }
console.log(__DECODE_0__(0));";

const AN_ARRAY_WITHOUT_A_DECODER: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
console.log(__STRING_ARRAY__());";

const AN_UNATTRIBUTED_DECODER: &str = "\
function __DECODE_0__(x) { return 'ZZZ'; }
function a1() { var h = ['log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
console[b1(0x0)](b1(0x1));
console.log(__DECODE_0__(0x0));";

const A_LABELLED_DECODER: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
l: function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); return e[c]; }
console[__DECODE_0__(0)]();";

const A_CONTROL_FLOW_OBJECT_MISSING_A_KEY: &str =
    "function hi() {\n  var obj = { rLxJs: '6|0|4' };\n  console.log(obj.rLxJs, obj.zzzzz);\n}";

const A_DISPATCH_OBJECT_WRITTEN_OUTSIDE_ITS_DEAD_BRANCH: &str = "function hi() {\n  var obj = { rLxJs: '6|0|4', aokfw: function (a, b) { return a === b; } };\n  if (obj.aokfw('aa', 'bb')) {\n    obj = { rLxJs: 'x' };\n  }\n  sink(obj);\n  console.log(obj.rLxJs);\n}";

const A_DEAD_WRITE_INSIDE_A_LIVE_STATEMENT: &str = "function hi() {\n  var obj = { rLxJs: '6|0|4', aokfw: function (a, b) { return a === b; } };\n  obj.aokfw('aa', 'bb') ? obj = { rLxJs: 'x' } : 0;\n  console.log(obj.rLxJs);\n}";

const A_LOGICAL_FORWARDER_GIVEN_AN_UNRESOLVED_ARGUMENT: &str = "function hi() {\n  var obj = { aokfw: function (a, b) { return a || b; } };\n  console.log(obj.aokfw(p, q));\n}";

const A_SELF_DEFENDING_CONTROLLER_THAT_ESCAPES: &str = "var e = (function () {\n  var f = !![];\n  return function (g, h) {\n    var i = f ? function () {\n      if (h) {\n        var j = h['apply'](g, arguments);\n        h = null;\n        return j;\n      }\n    } : function () {};\n    f = ![];\n    return i;\n  };\n})();\nconsole.log(typeof e);";

const A_DEBUG_PROTECTION_THAT_ESCAPES: &str = "function dpf(ret) {\n  function debuggerProtection(counter) {\n    if (typeof counter === 'string') {\n      return function (arg) {}['constructor']('while (true) {}')['apply']('counter');\n    } else {\n      debugger;\n    }\n    debuggerProtection(++counter);\n  }\n  try {\n    if (ret) {\n      return debuggerProtection;\n    } else {\n      debuggerProtection(0);\n    }\n  } catch (y) {}\n}\nconsole.log(typeof dpf);";

macro_rules! behaviour_unit {
    ($line:literal) => {
        concat!(
            "function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }\n",
            "function b(c, d) {\nvar e = a();\nreturn (b = function (f, g) {\nf = f - 0;\nvar i = e[f];\nreturn i;\n}), b(c, d);\n}\n",
            $line,
            "\nconsole.log(b(0));\nconsole.log(b(1));\nconsole.log(b(2));\nconsole.log(b(3));\n"
        )
    };
}

const A_STRING_ARRAY_THAT_ESCAPES: &str =
    include_str!("../../../../fixtures/behaviour/string-array-escapes.js");

const A_DECODER_USED_AS_A_VALUE: &str = behaviour_unit!("[1].map(b);");

const A_WITH_STATEMENT: &str = behaviour_unit!("with ({}) {}");

const A_DIRECT_EVAL: &str = behaviour_unit!("eval('1');");

const A_UNIT_IN_A_BLOCK_CALLED_OUTSIDE_IT: &str =
    "{ function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); return e[i]; } }
console.log(b(0), b(1), typeof a);";

const A_ROTATOR_IN_AN_UNCALLED_FUNCTION: &str =
    include_str!("../../../../fixtures/behaviour/06-rotator-in-uncalled-function.js");

const A_CALL_BEFORE_THE_ROTATOR: &str =
    include_str!("../../../../fixtures/behaviour/08-call-before-rotator.js");

const A_DECODER_BOUND_AFTER_THE_ROTATOR: &str =
    include_str!("../../../../fixtures/behaviour/var-decoder-declared-after-rotator.js");

const A_ROTATOR_THAT_DEFINES_A_GLOBAL: &str =
    include_str!("../../../../fixtures/behaviour/13-rotator-defines-global.js");

const A_NAME_SPELLED_IN_A_STRING: &str =
    include_str!("../../../../fixtures/behaviour/04-eval-names-array.js");

const A_DECODER_REASSIGNED: &str =
    include_str!("../../../../fixtures/behaviour/24-decoder-reassigned.js");

const AN_ARRAY_DECLARED_TWICE: &str =
    include_str!("../../../../fixtures/behaviour/redeclare-array.js");

const AN_ARRAY_NAME_DECLARED_IN_A_BLOCK: &str =
    include_str!("../../../../fixtures/behaviour/block-fn-array.js");

const AN_ARRAY_READING_A_DECLARED_UNDEFINED: &str = "(function(undefined){\nfunction qa(){ var h=['s0',undefined,'s2']; qa=function(){return h;}; return qa(); }\nfunction qb(i){ var e=qa(); return e[i] + '|'; }\nconsole.log(qb(0)); console.log(qb(1));\n})('X');";

const REACHABILITY_BUDGET: usize = 4096;

fn a_reachability_chain_past_the_budget() -> String {
    let links = REACHABILITY_BUDGET + 1;
    let mut body = String::from("var f0 = function () { return m.alpha; };\n");
    for index in 1..links {
        let previous = index - 1;
        body.push_str(&format!(
            "var f{index} = function () {{ return f{previous}(); }};\n"
        ));
    }
    let last = links - 1;
    body.push_str(&format!(
        "side();\nvar m = {{ alpha: 'x' }};\nreturn f{last}();\n"
    ));
    format!("function outer() {{\n{body}}}\nouter();\n")
}

pub enum Source {
    Whole(&'static str),
    Headed(&'static str),
    Built(fn() -> String),
    Superset(&'static str),
    DefaultLane(&'static str),
}

pub enum Match {
    Contains,
    Exact,
}

pub struct Case {
    pub source: Source,
    pub expects: &'static str,
    pub mode: Match,
}

pub fn program(source: &Source) -> String {
    match source {
        Source::Whole(text) | Source::Superset(text) | Source::DefaultLane(text) => {
            (*text).to_string()
        }
        Source::Headed(body) => format!("{HEAD}{body}"),
        Source::Built(build) => build(),
    }
}

pub fn stage_notes(case: &Case, source: &str) -> (String, Vec<Note>) {
    match case.source {
        Source::Superset(_) => crate::common::run_all_stage_notes_extended(source),
        Source::DefaultLane(_) => crate::common::run_all_stage_notes_default_lane(source),
        _ => crate::common::run_all_stage_notes(source),
    }
}

pub fn stages(case: &Case, source: &str) -> (String, Vec<String>) {
    let (code, notes) = stage_notes(case, source);
    (code, notes.into_iter().map(|note| note.message).collect())
}

pub const CASES: &[Case] = &[
    Case {
        source: Source::Whole(WRAPPED_UNIT),
        expects: notes::STRING_ARRAY_FOUND,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(WRAPPED_UNIT),
        expects: notes::ROTATOR_NONE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(WRAPPED_UNIT),
        expects: notes::DECODERS_FOUND,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(ROTATED_UNIT),
        expects: notes::ROTATOR_FOUND,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("var s = ['a'];\nconsole.log(s[5]);"),
        expects: notes::SIMPLE_ARRAY_ACCESS,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "(function(undefined){ var s = ['a', undefined];\nconsole.log(s[0] + s[1]); })('b');",
        ),
        expects: notes::SIMPLE_ARRAY_UNDEFINED_NOT_GLOBAL,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("var s = ['a', 'b'];\nconsole.log(s[0].length, s[1]);"),
        expects: notes::SIMPLE_ARRAY_READ_NOT_A_VALUE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("function f() { return s[0]; }\nf();\nvar s = ['a'];"),
        expects: notes::ARRAY_READ_BEFORE_DECLARATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function f() { var s = ['a', 'b']; eval('s.reverse()'); console.log(s[0]); }\nf();",
        ),
        expects: notes::SIMPLE_ARRAY_DIRECT_EVAL,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function f() { var s = ['a', 'b']; with ({}) { console.log(s[0]); } }\nf();",
        ),
        expects: notes::SIMPLE_ARRAY_WITH,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("var s = ['a', 'b'];\nthis.s.reverse();\nconsole.log(s[0]);"),
        expects: notes::SIMPLE_ARRAY_GLOBAL_REACH,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("var s = ['a', 'b'];\nconsole.log('s' in this, s[0]);"),
        expects: notes::SIMPLE_ARRAY_NAME_OBSERVED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "var s = ['a', 'b'];\n(function () {\n  console.log(s[0]);\n  {\n    function s() {}\n  }\n})();",
        ),
        expects: notes::BLOCK_FUNCTION_NAME,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "var s = ['a', 'b'];\nvar t = s;\nconsole.log('t' in this, t[0]);",
        ),
        expects: notes::ARRAY_ALIAS_NAME_OBSERVED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function g() { return b[0]; }\nconsole.log(g());\nvar a = ['log'];\nvar b = a;",
        ),
        expects: notes::ARRAY_ALIAS_READ_BEFORE_DECLARATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "var a = ['log'];\nvar b = a;\nconsole[b[0]]();\nfunction f() {\n  var a = 1;\n  return a;\n}",
        ),
        expects: notes::ARRAY_ALIAS_NAME_NOT_UNIQUE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("var a = ['log'];\nvar b = a;\nfn(b);\nconsole[b[0]]();"),
        expects: notes::ARRAY_ALIAS_ESCAPES,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_CONTROL_FLOW_OBJECT_MISSING_A_KEY),
        expects: notes::OBJECT_MISSING_PROP,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("console.log(({ aaa: 1, aaa: 2 }).aaa, __DECODE_0__(0));"),
        expects: notes::OBJECT_AMBIGUOUS_KEY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function f() { return o.x; }\nf();\nvar o = { x: 1 };\nconsole.log(o.x, __DECODE_0__(0));",
        ),
        expects: notes::OBJECT_READ_BEFORE_DECLARATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("var alias = __DECODE_0__;\nvar alias = __DECODE_0__;\nalias(0);"),
        expects: notes::WRAPPER_ALIAS_NOT_REMOVABLE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("var alias = __DECODE_0__;\nalias = q;\nalias(0);"),
        expects: notes::WRAPPER_ALIAS_NOT_CONSTANT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("var alias = 1;\nalias = __DECODE_0__;\nalias(0);"),
        expects: notes::WRAPPER_ALIAS_ALREADY_INITIALISED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("var alias;\nalias = __DECODE_0__;\nalias = q;\nalias(0);"),
        expects: notes::WRAPPER_ALIAS_EXTRA_WRITES,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function g() { return alias(0x0, 0x1); }\nconsole[g()]('t');\nvar alias = __DECODE_0__;",
        ),
        expects: notes::WRAPPER_ALIAS_READ_BEFORE_THE_ALIAS,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "var alias;\nalias = __DECODE_0__;\nf(alias = alias);\nconsole[alias(0)]();",
        ),
        expects: notes::WRAPPER_ALIAS_SELF_ASSIGNMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function alias(a, b) { return __DECODE_0__(a - 1, b); }\nf(alias(alias(g(), 3), 4));",
        ),
        expects: notes::WRAPPER_SITE_KEPT,
        mode: Match::Exact,
    },
    Case {
        source: Source::Headed(
            "function alias(a, b) { return __DECODE_0__(a - 1, b); }\nf(alias(alias(g(), 3), 4));",
        ),
        expects: notes::WRAPPER_DEFERRED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function w(a, b) { return __DECODE_0__(a, b); }\nw = function () { return 1; };\nw(1, 2);",
        ),
        expects: notes::WRAPPER_BINDING_UNSTABLE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "f(w(1, 2));\nvar w = function (a, b) { return __DECODE_0__(a, b); };\nw(3, 4);",
        ),
        expects: notes::WRAPPER_READ_BEFORE_DECLARATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("function w(a, b) { return __DECODE_0__(a, class {}); }\nw(1, 2);"),
        expects: notes::WRAPPER_NESTED_FUNCTION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function alias(a, b) { return __DECODE_0__(b - 938, a); }\nalias(function () { return 1; }, 5);",
        ),
        expects: notes::WRAPPER_SITE_SCOPED_ARGUMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function w(x, y) { return __DECODE_0__(x + y, x); }\nvar i = 0x0;\nconsole[w(i++, 0x0)]('t');",
        ),
        expects: notes::WRAPPER_SITE_IMPURE_ARGUMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "var s = unknownGlobal;\nfunction w(x, y) { return __DECODE_0__(x + s, y); }\nfunction caller() { var s = 0x1; return w(0x0, 0x0); }\nconsole[caller()]('t');",
        ),
        expects: notes::WRAPPER_BODY_NOT_SELF_CONTAINED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function alias(a, b) { return __DECODE_0__(b - 938, a); }\nf(alias);\nalias(4, 5);",
        ),
        expects: notes::WRAPPER_PARTIAL,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("console.log(__DECODE_0__(p ? function () {} : 0xc8));"),
        expects: notes::TERNARY_HOLDS_A_SCOPE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("var k = side();\nconsole.log(__DECODE_0__(k));"),
        expects: notes::NO_LITERAL_INITIALISER,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "let x;\nconsole.log(__DECODE_0__(x));\nx = 0x0;\nconsole.log(__DECODE_0__(x));",
        ),
        expects: notes::NO_PRECEDING_ASSIGNMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("let x;\nif (c) x = 0xd6;\nconsole.log(__DECODE_0__(x));"),
        expects: notes::ASSIGNMENT_NOT_BESIDE_THE_DECLARATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "function g() { return __DECODE_0__(k); }\nconsole.log(g());\nvar k = 0x1;",
        ),
        expects: notes::USE_BEFORE_INITIALISATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed("let x;\nx = 0xd6;\nx = g();\nconsole.log(__DECODE_0__(x));"),
        expects: notes::NON_LITERAL_WRITE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Headed(
            "var k;\nk = 0x1;\nvar f = function () { return __DECODE_0__(k); };\nf();",
        ),
        expects: notes::REFERENCE_NOT_DOMINATED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_THAT_BAILS),
        expects: notes::DECODE_BAILED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_THAT_SPENDS_THE_HEAP_BUDGET),
        expects: notes::DECODE_BUDGET_SPENT,
        mode: Match::Exact,
    },
    Case {
        source: Source::Whole(A_MUTATING_DECODER),
        expects: notes::DECODER_NOT_READ_ONLY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_READING_OUTER_STATE),
        expects: notes::DECODER_READS_OUTER_BINDING,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_BUILT_IN_REPLACED),
        expects: notes::BUILTIN_PATCHED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODED_VALUE_SPELLING_THE_DECODER),
        expects: notes::DECODED_NAME,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_REFUSED_UNITS_ORIGINAL_NAME_IS_A_FREE_REFERENCE),
        expects: notes::UNSTAMP_FREE_REFERENCE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(TWO_REFUSED_UNITS_WHOSE_ORIGINAL_NAMES_COLLIDE),
        expects: notes::UNSTAMP_BLOCKED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_OUT_OF_RANGE_INDEX),
        expects: notes::DECODE_ERROR,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_OUT_OF_RANGE_INDEX),
        expects: notes::NON_STRING_RESULTS,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_OF_ARRAYS),
        expects: notes::NON_PRIMITIVE_RESULT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_OF_ARRAYS),
        expects: notes::DECODED_CALL_SITES,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_WITHOUT_A_DECODER),
        expects: notes::MACHINERY_INCOMPLETE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_UNATTRIBUTED_DECODER),
        expects: notes::UNATTRIBUTED_DECODER,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_STRING_ARRAY_THAT_ESCAPES),
        expects: notes::STRING_ARRAY_ESCAPES,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_ROTATOR_IN_AN_UNCALLED_FUNCTION),
        expects: notes::ROTATOR_NOT_TOP_LEVEL,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_CALL_BEFORE_THE_ROTATOR),
        expects: notes::CODE_BEFORE_ROTATOR,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_BOUND_AFTER_THE_ROTATOR),
        expects: notes::MACHINERY_NOT_INITIALISED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_ROTATOR_THAT_DEFINES_A_GLOBAL),
        expects: notes::ROTATOR_NOT_SEALED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_USED_AS_A_VALUE),
        expects: notes::DECODER_USED_AS_VALUE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_WITH_STATEMENT),
        expects: notes::WITH_STATEMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DIRECT_EVAL),
        expects: notes::DIRECT_EVAL_SCOPE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_UNIT_IN_A_BLOCK_CALLED_OUTSIDE_IT),
        expects: notes::MACHINERY_IN_A_BLOCK,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_NAME_SPELLED_IN_A_STRING),
        expects: notes::MACHINERY_NAME_SPELLED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DECODER_REASSIGNED),
        expects: notes::DECODER_WRITTEN_OUTSIDE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_DECLARED_TWICE),
        expects: notes::MACHINERY_REDECLARED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_NAME_DECLARED_IN_A_BLOCK),
        expects: notes::MACHINERY_REDECLARED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_ARRAY_READING_A_DECLARED_UNDEFINED),
        expects: notes::STRING_ARRAY_DECLARED_UNDEFINED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(AN_UNDECODABLE_CALL),
        expects: notes::MACHINERY_REFERENCES_KEPT,
        mode: Match::Contains,
    },
    Case {
        source: Source::DefaultLane(A_UNIT_REACHED_THROUGH_THE_GLOBAL_OBJECT),
        expects: notes::STRING_ARRAY_GLOBAL_REACH,
        mode: Match::Contains,
    },
    Case {
        source: Source::DefaultLane(A_BOOTSTRAP_WHOSE_TABLE_IS_WRITTEN),
        expects: notes::WEBPACK_BOOTSTRAP_UNPROVEN,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_UNIT_WHOSE_GLOBAL_NAMES_ARE_LISTED),
        expects: notes::MACHINERY_NAME_OBSERVED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_GLOBAL_HANDED_TO_A_REQUIRED_PATH),
        expects: notes::MACHINERY_REMOVED_UNDER_OUTSIDE_PREMISE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_LABELLED_DECODER),
        expects: notes::MACHINERY_NOT_A_LIST_MEMBER,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_CONTROL_FLOW_OBJECT_MISSING_A_KEY),
        expects: notes::CONTROL_FLOW_MISSING_PROP,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_CONTROL_FLOW_OBJECT_MISSING_A_KEY),
        expects: notes::CONTROL_FLOW_OBJECT_PARTIAL,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var obj = { aaaaa: '1' };\n  obj['aaaaa'] = '2';\n  var alias = obj;\n  console.log(alias.aaaaa);\n}",
        ),
        expects: notes::CONTROL_FLOW_DUPLICATE_KEY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("function hi(k) {\n  console.log({ [k]: 'aaa', bbbbb: 'x' }.bbbbb);\n}"),
        expects: notes::CONTROL_FLOW_NON_CONSTANT_KEY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var obj = { aaaaa: function (x, y) { return x + y; } };\n  return obj.aaaaa;\n}",
        ),
        expects: notes::CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function f() { return o.aaaaa; }\nf();\nvar o = { aaaaa: 'x' };\nconsole.log(o.aaaaa);",
        ),
        expects: notes::CONTROL_FLOW_USE_BEFORE_INIT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset(A_DISPATCH_OBJECT_WRITTEN_OUTSIDE_ITS_DEAD_BRANCH),
        expects: notes::CONTROL_FLOW_LIVE_MUTATION,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset(A_DEAD_WRITE_INSIDE_A_LIVE_STATEMENT),
        expects: notes::CONTROL_FLOW_DEAD_WRITE_KEPT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset(A_LOGICAL_FORWARDER_GIVEN_AN_UNRESOLVED_ARGUMENT),
        expects: notes::LOGICAL_FORWARDER_ARGUMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var d = '0|9'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n    }\n    break;\n  }\n}",
        ),
        expects: notes::SWITCH_SEQUENCE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        ),
        expects: notes::SWITCH_FALLTHROUGH,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var d = '0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        break;\n      case '1':\n        b();\n        continue;\n    }\n    break;\n  }\n}",
        ),
        expects: notes::SWITCH_CASE_EXIT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(
            "function hi() {\n  var d = '0'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        let x = 2;\n        a(x);\n        continue;\n    }\n    break;\n  }\n  let x = 1;\n  b(x);\n}",
        ),
        expects: notes::SWITCH_CASE_SHADOW,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("log(o);\nconst o = {};\no.a = 1;"),
        expects: notes::MERGE_USE_BEFORE_INIT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("atob(\"-\");"),
        expects: notes::GLOBAL_NOT_EVALUATED,
        mode: Match::Contains,
    },
    Case {
        source: Source::Built(a_reachability_chain_past_the_budget),
        expects: notes::INIT_ORDER_BUDGET,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_SELF_DEFENDING_CONTROLLER_THAT_ESCAPES),
        expects: notes::SELF_DEFENDING_REFERENCE_KEPT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole(A_DEBUG_PROTECTION_THAT_ESCAPES),
        expects: notes::DEBUG_PROTECTION_REFERENCE_KEPT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("with (globalThis) { console.log(1); }"),
        expects: notes::SCOPE_HAZARD,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("const obj = {};\nobj[key] = 1;\nlog(obj);"),
        expects: notes::MERGE_ADOPTED_DYNAMIC_KEY,
        mode: Match::Contains,
    },
    Case {
        source: Source::Whole("console.log(1 + 2);"),
        expects: notes::NO_STRING_ARRAY_NOTE,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset("function f(c) {\n  c = 2;\n  return arguments[0];\n}"),
        expects: notes::VAR_FUNCTION_READS_ARGUMENTS,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset(
            "function f(c) {\n  c = 2;\n  return g();\n}\nfunction g() {\n  return f[\"arguments\"][0];\n}",
        ),
        expects: notes::VAR_ARGUMENTS_PROPERTY_ALIAS,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset("function f(c) {\n  c = 1;\n  c = 2;\n  return c;\n}"),
        expects: notes::VAR_PARAMETER_WRITTEN_AGAIN,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset("function f(c) {\n  console.log(c);\n  c = 2;\n}"),
        expects: notes::VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT,
        mode: Match::Contains,
    },
    Case {
        source: Source::Superset(
            "function h() {\n  var y;\n  Func(y);\n  y = \"z\";\n  return y;\n}",
        ),
        expects: notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE,
        mode: Match::Contains,
    },
    Case {
        source: Source::DefaultLane(
            "function g() {\n  if (\"a\" !== \"a\") {\n    {\n      function f() {}\n    }\n  }\n  return f;\n}",
        ),
        expects: notes::DEAD_CODE_DISCARDED_FUNCTION,
        mode: Match::Contains,
    },
];
