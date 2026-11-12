use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::node::assert_node_agrees;
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

const BABEL: &str = include_str!("data/class-async/babel.js");
const SWC: &str = include_str!("data/class-async/swc.js");
const AWAITER: &str = include_str!("data/class-async/tslib-awaiter.js");
const GENERATOR: &str = include_str!("data/class-async/tslib-generator.js");

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn unsafe_run(source: &str) -> String {
    let alloc = Allocator::default();
    let options = PassOptions {
        unsafe_transforms: true,
        extended: true,
        ..PassOptions::default()
    };
    transform_report(&alloc, source, &ctors(), options).0
}

fn row(helpers: &[&str], body: &str) -> String {
    run_row(format!("{}\n{body}", helpers.join("\n")))
}

fn strict_row(helpers: &[&str], body: &str) -> String {
    run_row(format!("\"use strict\";\n{}\n{body}", helpers.join("\n")))
}

fn run_row(source: String) -> String {
    let code = unsafe_run(&source);
    assert_node_agrees(&source, &code);
    code
}

fn has(code: &str, part: &str) {
    assert!(code.contains(part), "missing {part:?} in:\n{code}");
}

fn lacks(code: &str, part: &str) {
    assert!(!code.contains(part), "unexpected {part:?} in:\n{code}");
}

const SLOPPY_HERITAGE: &str = r#"
function mix(f) { f.prototype.sloppy = f(); return f; }
var K3 = /*#__PURE__*/function (_ref) {
  "use strict";
  function K3() {
    _classCallCheck(this, K3);
    return _callSuper(this, K3, arguments);
  }
  _inherits(K3, _ref);
  return _createClass(K3);
}(mix(function Q() { return typeof this; }));
console.log(K3.prototype.sloppy);
"#;

#[test]
fn a_sloppy_function_in_the_parent_argument_keeps_the_class_helpers() {
    let code = row(&[BABEL], SLOPPY_HERITAGE);
    has(&code, "_inherits(K3");
}

#[test]
fn a_strict_file_moves_a_function_parent_argument_into_extends() {
    let code = strict_row(&[BABEL], SLOPPY_HERITAGE);
    has(&code, "class K3 extends mix(function Q()");
}

#[test]
fn a_sloppy_file_still_moves_a_plain_parent_into_extends() {
    let code = row(
        &[BABEL],
        r#"
var ns = { Par: function Par() { this.p = 1; } };
var K = /*#__PURE__*/function (_ns$Par) {
  "use strict";
  function K() {
    _classCallCheck(this, K);
    return _callSuper(this, K, arguments);
  }
  _inherits(K, _ns$Par);
  return _createClass(K);
}(ns.Par);
console.log(new K().p);
"#,
    );
    has(&code, "class K extends ns.Par");
}

const CONCAT: &str = r#"
function Par(a, b) { this.args = [].slice.call(arguments); }
var K1 = /*#__PURE__*/function (_Par) {
  "use strict";
  function K1(x) {
    var _this;
    _classCallCheck(this, K1);
    _this = _callSuper(this, K1, [].concat(x));
    return _this;
  }
  _inherits(K1, _Par);
  return _createClass(K1);
}(Par);
function t(f) { try { return JSON.stringify(f().args); } catch (e) { return "!" + e.constructor.name; } }
console.log(t(() => new K1("ab")), t(() => new K1([[1], 2])), t(() => new K1(7)));
"#;

#[test]
fn concat_of_an_unknown_value_is_not_spread_into_super() {
    let code = row(&[BABEL], CONCAT);
    lacks(&code, "super(...x)");
}

#[test]
fn concat_of_babels_rest_array_is_spread_into_super() {
    let code = row(
        &[BABEL],
        r#"
function Par(a, b) { this.args = [].slice.call(arguments); }
var K8 = /*#__PURE__*/function (_Par) {
  "use strict";
  function K8() {
    var _this;
    _classCallCheck(this, K8);
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }
    _this = _callSuper(this, K8, [].concat(args));
    _defineProperty(_this, "n", args.length);
    return _this;
  }
  _inherits(K8, _Par);
  return _createClass(K8);
}(Par);
var k = new K8(1, [2], 3);
console.log(JSON.stringify(k.args) + k.n + K8.length);
"#,
    );
    has(&code, "super(...args)");
}

#[test]
fn a_this_in_nested_class_heritage_keeps_the_derived_constructor() {
    let code = strict_row(
        &[BABEL],
        r#"
function Other() { return { other: 1 }; }
var K9 = /*#__PURE__*/function (_Other) {
  function K9() {
    var _this;
    _classCallCheck(this, K9);
    _this = _callSuper(this, K9);
    _this.K = class extends (this.constructor === K9 ? Object : Array) {};
    return _this;
  }
  _inherits(K9, _Other);
  return _createClass(K9);
}(Other);
console.log(Object.getPrototypeOf(new K9().K) === Object);
"#,
    );
    has(&code, "_callSuper(this, K9)");
}

#[test]
fn a_this_in_a_nested_computed_key_keeps_the_derived_constructor() {
    let code = strict_row(
        &[BABEL],
        r#"
function Other() { return { other: 1 }; }
var K10 = /*#__PURE__*/function (_Other) {
  function K10() {
    var _this;
    _classCallCheck(this, K10);
    _this = _callSuper(this, K10);
    _this.K = class { [String(this.other)]() { return 1; } };
    return _this;
  }
  _inherits(K10, _Other);
  return _createClass(K10);
}(Other);
console.log(Object.getOwnPropertyNames(new K10().K.prototype).join());
"#,
    );
    has(&code, "_callSuper(this, K10)");
}

#[test]
fn a_this_inside_a_nested_class_body_still_converts() {
    let code = strict_row(
        &[BABEL],
        r#"
function Par() { this.p = 1; }
var K11 = /*#__PURE__*/function (_Par) {
  function K11() {
    var _this;
    _classCallCheck(this, K11);
    _this = _callSuper(this, K11);
    _this.K = class { static x = this; m() { return this; } };
    return _this;
  }
  _inherits(K11, _Par);
  return _createClass(K11);
}(Par);
var k = new K11();
console.log(k.K.x === k.K, k.p);
"#,
    );
    has(&code, "class K11 extends Par");
}

#[test]
fn a_returned_open_create_class_outside_a_declaration_keeps_the_helpers() {
    let code = row(
        &[SWC],
        r#"
var holder = {};
holder.C = function () {
    "use strict";
    function C() { _class_call_check(this, C); }
    return _create_class(C, [{ key: "m", value: function m() { return 1; } }]);
}();
var D = holder.C;
D.prototype = { m: function () { return 2; } };
console.log(new D().m());
"#,
    );
    has(&code, "_class_call_check(this, C)");
}

#[test]
fn a_direct_open_create_class_outside_a_declaration_keeps_the_helpers() {
    let code = row(
        &[SWC],
        r#"
function keep(x) { keep.last = x; return x; }
keep(_create_class(function E() { "use strict"; _class_call_check(this, E); }, [{ key: "m", value: function m() { "use strict"; return 1; } }]));
var F = keep.last;
F.prototype = { m: function () { return 2; } };
console.log(new F().m());
"#,
    );
    has(&code, "_class_call_check(this, E)");
}

#[test]
fn swcs_own_class_shapes_still_convert() {
    let code = row(
        &[SWC],
        r#"
var G = function () {
    "use strict";
    function G() { _class_call_check(this, G); }
    _create_class(G, [{ key: "m", value: function m() { return 1; } }]);
    return G;
}();
var E = _create_class(function E() { "use strict"; _class_call_check(this, E); }, [{ key: "m", value: function m() { "use strict"; return 2; } }]);
console.log(new G().m(), new E().m());
"#,
    );
    has(&code, "var G = class G {");
    has(&code, "var E = class E {");
}

#[test]
fn an_undefined_receiver_converts_the_awaiter() {
    let code = row(
        &[AWAITER],
        r#"
const c1 = (x) => __awaiter(void 0, void 0, void 0, function* () { return (yield x) + 1; });
const c2 = function (x) { return __awaiter(void 0, void 0, void 0, function* () { return (yield x) + 1; }); };
const c3 = (x) => __awaiter(undefined, void 0, void 0, function* () { return (yield x) + 1; });
Promise.all([c1(1), c2(2), c3(3)]).then((v) => console.log(v.join()));
"#,
    );
    lacks(&code, "__awaiter(");
}

#[test]
fn an_undefined_receiver_keeps_the_awaiter_when_the_body_reads_this() {
    let code = strict_row(
        &[AWAITER],
        r#"
const c4 = function () { return __awaiter(void 0, void 0, void 0, function* () { return typeof this; }); };
c4.call({}).then((v) => console.log(v));
"#,
    );
    has(&code, "__awaiter(");
}

#[test]
fn a_forwarded_parameter_default_reading_this_keeps_the_awaiter() {
    let code = strict_row(
        &[AWAITER],
        r#"
var b6 = function (x) { return __awaiter(null, arguments, void 0, function* (y, z = this === null) { return z; }); };
b6.call({}, 1).then(function (v) { console.log(v); });
"#,
    );
    has(&code, "__awaiter(");
}

#[test]
fn an_outer_binding_named_await_keeps_the_awaiter() {
    let code = row(
        &[AWAITER],
        r#"
function a1(await) { return __awaiter(this, void 0, void 0, function* () { return (yield 1) + 1; }); }
var a4 = function (x) { var await; return __awaiter(this, void 0, void 0, function* () { return (yield 1) + 1; }); };
a1(1).then(function (v) { return a4().then(function (w) { console.log(v, w); }); });
"#,
    );
    lacks(&code, "async function");
}

#[test]
fn an_outer_binding_named_yield_keeps_the_generator() {
    let code = row(
        &[GENERATOR],
        r#"
function g1(a, b) { var yield; return __generator(this, function (_a) { switch (_a.label) { case 0: return [4, b]; case 1: _a.sent(); return [2, b]; } }); }
var it = g1(1, 2);
console.log(it.next().value, it.next().value);
"#,
    );
    lacks(&code, "function*");
}

const OWN_START: &str = r#"
var log = [];
function ownStart(n) {
    var i;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                i = 0;
                _a.label = 1;
            case 1:
                _a.trys.push([1, , 3, 4]);
                i++;
                if (i < n) return [3 /*break*/, 1];
                return [4 /*yield*/, "y" + i];
            case 2:
                _a.sent();
                return [3 /*break*/, 4];
            case 3:
                log.push("fin" + i);
                return [7 /*endfinally*/];
            case 4: return [2 /*return*/, i];
        }
    });
}
var it = ownStart(3);
var first = it.next();
var last = it.next();
console.log(first.value, last.value, log.join());
"#;

#[test]
fn a_jump_to_the_tries_own_start_keeps_the_generator() {
    let code = row(&[GENERATOR], OWN_START);
    lacks(&code, "function*");
}
