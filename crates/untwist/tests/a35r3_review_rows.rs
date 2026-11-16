use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const PROGRAMS: &[(&str, &str)] = &[
    (
        "f1-catch-if-function",
        r##"(function(){ var e = {abcde: "x", fghij: "q"}; eval("try { throw 1 } catch (e) { if (1) function e(){} }"); console.log(typeof e, e && e.fghij); })();"##,
    ),
    (
        "f1-rename",
        r##"(function(){ var e = "outer"; (function(){ eval("try { throw 1 } catch (e) { if (0) ; else function e(){} }"); console.log(typeof e); })(); console.log(typeof e); })();"##,
    ),
    (
        "f3-sg01",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(typeof globalThis.g);"##,
    ),
    (
        "f3-sg02",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(typeof this.g);"##,
    ),
    (
        "f3-sg03",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((0, eval)("typeof g"));"##,
    ),
    (
        "f3-sg04",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(new Function("return typeof h")());"##,
    ),
    (
        "f3-sg05",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log("g" in globalThis);"##,
    ),
    (
        "f3-sg06",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(Object.keys(globalThis).includes("g"));"##,
    ),
    (
        "f3-sg07",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(typeof globalThis["g"]);"##,
    ),
    (
        "f3-sg08",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((function(){ return typeof this.h })());"##,
    ),
    (
        "f3-sg09",
        r##"const g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); const h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((0, eval)("typeof g"));"##,
    ),
    (
        "f3-sg10",
        r##"var dpf = function (ret) {
  function debuggerProtection(counter) {
    if (typeof counter === "string") {
      return function (arg) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + counter / counter).length !== 1 || counter % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
};
console.log(typeof globalThis.dpf, (0, eval)("typeof dpf"));"##,
    ),
    (
        "f3-sg11",
        r##"function dpf(ret) {
  function debuggerProtection(counter) {
    if (typeof counter === "string") {
      return function (arg) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + counter / counter).length !== 1 || counter % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
}
console.log(typeof globalThis.dpf, (0, eval)("typeof dpf"));"##,
    ),
    (
        "f3-se01",
        r##"const g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); const h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((0, eval)("typeof g"), new Function("return typeof h")(), typeof globalThis.g);"##,
    ),
    (
        "f3-se02",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((0, eval)("typeof g"), new Function("return typeof h")(), typeof globalThis.g, "h" in globalThis, typeof this.g);"##,
    ),
    (
        "f3-selfdefending",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log(typeof globalThis.g);"##,
    ),
    (
        "f3-indirect-eval",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log((0, eval)("typeof g"));"##,
    ),
    (
        "f3-debug",
        r##"function dpf(ret) {
  function debuggerProtection(counter) {
    if (typeof counter === "string") {
      return function (arg) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + counter / counter).length !== 1 || counter % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
}
console.log(typeof globalThis.dpf, (0, eval)("typeof dpf"));"##,
    ),
    (
        "x-sa1",
        r##"var _0x3e7f27 = _0x331b;
(function (_0x5d54cf, _0x30d774) {
    var _0xa56e53 = _0x331b, _0x4f3734 = _0x5d54cf();
    while (!![]) {
        try {
            var _0x1bdc8b = -parseInt(_0xa56e53(0x107)) / 0x1 * (parseInt(_0xa56e53(0x103)) / 0x2) + parseInt(_0xa56e53(0x10b)) / 0x3 + parseInt(_0xa56e53(0x10d)) / 0x4 * (-parseInt(_0xa56e53(0x112)) / 0x5) + parseInt(_0xa56e53(0x10e)) / 0x6 * (parseInt(_0xa56e53(0x106)) / 0x7) + -parseInt(_0xa56e53(0x104)) / 0x8 * (parseInt(_0xa56e53(0x105)) / 0x9) + parseInt(_0xa56e53(0x10a)) / 0xa + -parseInt(_0xa56e53(0x10f)) / 0xb * (-parseInt(_0xa56e53(0x109)) / 0xc);
            if (_0x1bdc8b === _0x30d774)
                break;
            else
                _0x4f3734['push'](_0x4f3734['shift']());
        } catch (_0x1535e5) {
            _0x4f3734['push'](_0x4f3734['shift']());
        }
    }
}(_0x36aa, 0x19c84));
function _0x36aa() {
    var _0x105156 = [
        'bye\x20',
        'length',
        '18565vYFuqh',
        '906PwOnsD',
        '8CXbWuh',
        '174906Exjawe',
        '245tdvqso',
        '216wOpALZ',
        'world',
        '1270716pkdJfp',
        '493890OuCaoM',
        '78327KYtVMT',
        'hello\x20',
        '160TozpVh',
        '32574gvfDPf',
        '11rYtAdN'
    ];
    _0x36aa = function () {
        return _0x105156;
    };
    return _0x36aa();
}
function _0x331b(_0x365bd4, _0x49df1e) {
    _0x365bd4 = _0x365bd4 - 0x103;
    var _0x36aa33 = _0x36aa();
    var _0x331b82 = _0x36aa33[_0x365bd4];
    return _0x331b82;
}
function greet(_0x39f9ac) {
    var _0x5d5fc6 = _0x331b, _0x20e089 = _0x5d5fc6(0x10c) + _0x39f9ac, _0x5f2e6b = _0x5d5fc6(0x110) + _0x39f9ac;
    if (_0x39f9ac[_0x5d5fc6(0x111)] > 0x2)
        return _0x20e089 + '!' + _0x5f2e6b;
    return _0x5f2e6b + '?' + _0x20e089;
}
console.log(greet(_0x3e7f27(0x108)), greet('x'));
console.log(typeof globalThis._0x331b, typeof globalThis._0x36aa);"##,
    ),
    (
        "x-sa2",
        r##"var _0x3e7f27 = _0x331b;
(function (_0x5d54cf, _0x30d774) {
    var _0xa56e53 = _0x331b, _0x4f3734 = _0x5d54cf();
    while (!![]) {
        try {
            var _0x1bdc8b = -parseInt(_0xa56e53(0x107)) / 0x1 * (parseInt(_0xa56e53(0x103)) / 0x2) + parseInt(_0xa56e53(0x10b)) / 0x3 + parseInt(_0xa56e53(0x10d)) / 0x4 * (-parseInt(_0xa56e53(0x112)) / 0x5) + parseInt(_0xa56e53(0x10e)) / 0x6 * (parseInt(_0xa56e53(0x106)) / 0x7) + -parseInt(_0xa56e53(0x104)) / 0x8 * (parseInt(_0xa56e53(0x105)) / 0x9) + parseInt(_0xa56e53(0x10a)) / 0xa + -parseInt(_0xa56e53(0x10f)) / 0xb * (-parseInt(_0xa56e53(0x109)) / 0xc);
            if (_0x1bdc8b === _0x30d774)
                break;
            else
                _0x4f3734['push'](_0x4f3734['shift']());
        } catch (_0x1535e5) {
            _0x4f3734['push'](_0x4f3734['shift']());
        }
    }
}(_0x36aa, 0x19c84));
function _0x36aa() {
    var _0x105156 = [
        'bye\x20',
        'length',
        '18565vYFuqh',
        '906PwOnsD',
        '8CXbWuh',
        '174906Exjawe',
        '245tdvqso',
        '216wOpALZ',
        'world',
        '1270716pkdJfp',
        '493890OuCaoM',
        '78327KYtVMT',
        'hello\x20',
        '160TozpVh',
        '32574gvfDPf',
        '11rYtAdN'
    ];
    _0x36aa = function () {
        return _0x105156;
    };
    return _0x36aa();
}
function _0x331b(_0x365bd4, _0x49df1e) {
    _0x365bd4 = _0x365bd4 - 0x103;
    var _0x36aa33 = _0x36aa();
    var _0x331b82 = _0x36aa33[_0x365bd4];
    return _0x331b82;
}
function greet(_0x39f9ac) {
    var _0x5d5fc6 = _0x331b, _0x20e089 = _0x5d5fc6(0x10c) + _0x39f9ac, _0x5f2e6b = _0x5d5fc6(0x110) + _0x39f9ac;
    if (_0x39f9ac[_0x5d5fc6(0x111)] > 0x2)
        return _0x20e089 + '!' + _0x5f2e6b;
    return _0x5f2e6b + '?' + _0x20e089;
}
console.log(greet(_0x3e7f27(0x108)), greet('x'));
console.log((0, eval)("typeof _0x331b + typeof _0x36aa + typeof _0x3e7f27"));"##,
    ),
    (
        "x-sa3",
        r##"var _0x3e7f27 = _0x331b;
(function (_0x5d54cf, _0x30d774) {
    var _0xa56e53 = _0x331b, _0x4f3734 = _0x5d54cf();
    while (!![]) {
        try {
            var _0x1bdc8b = -parseInt(_0xa56e53(0x107)) / 0x1 * (parseInt(_0xa56e53(0x103)) / 0x2) + parseInt(_0xa56e53(0x10b)) / 0x3 + parseInt(_0xa56e53(0x10d)) / 0x4 * (-parseInt(_0xa56e53(0x112)) / 0x5) + parseInt(_0xa56e53(0x10e)) / 0x6 * (parseInt(_0xa56e53(0x106)) / 0x7) + -parseInt(_0xa56e53(0x104)) / 0x8 * (parseInt(_0xa56e53(0x105)) / 0x9) + parseInt(_0xa56e53(0x10a)) / 0xa + -parseInt(_0xa56e53(0x10f)) / 0xb * (-parseInt(_0xa56e53(0x109)) / 0xc);
            if (_0x1bdc8b === _0x30d774)
                break;
            else
                _0x4f3734['push'](_0x4f3734['shift']());
        } catch (_0x1535e5) {
            _0x4f3734['push'](_0x4f3734['shift']());
        }
    }
}(_0x36aa, 0x19c84));
function _0x36aa() {
    var _0x105156 = [
        'bye\x20',
        'length',
        '18565vYFuqh',
        '906PwOnsD',
        '8CXbWuh',
        '174906Exjawe',
        '245tdvqso',
        '216wOpALZ',
        'world',
        '1270716pkdJfp',
        '493890OuCaoM',
        '78327KYtVMT',
        'hello\x20',
        '160TozpVh',
        '32574gvfDPf',
        '11rYtAdN'
    ];
    _0x36aa = function () {
        return _0x105156;
    };
    return _0x36aa();
}
function _0x331b(_0x365bd4, _0x49df1e) {
    _0x365bd4 = _0x365bd4 - 0x103;
    var _0x36aa33 = _0x36aa();
    var _0x331b82 = _0x36aa33[_0x365bd4];
    return _0x331b82;
}
function greet(_0x39f9ac) {
    var _0x5d5fc6 = _0x331b, _0x20e089 = _0x5d5fc6(0x10c) + _0x39f9ac, _0x5f2e6b = _0x5d5fc6(0x110) + _0x39f9ac;
    if (_0x39f9ac[_0x5d5fc6(0x111)] > 0x2)
        return _0x20e089 + '!' + _0x5f2e6b;
    return _0x5f2e6b + '?' + _0x20e089;
}
console.log(greet(_0x3e7f27(0x108)), greet('x'));
console.log("_0x36aa" in globalThis, "_0x3e7f27" in this);"##,
    ),
    (
        "x-sa4",
        r##"var _0x3e7f27 = _0x331b;
(function (_0x5d54cf, _0x30d774) {
    var _0xa56e53 = _0x331b, _0x4f3734 = _0x5d54cf();
    while (!![]) {
        try {
            var _0x1bdc8b = -parseInt(_0xa56e53(0x107)) / 0x1 * (parseInt(_0xa56e53(0x103)) / 0x2) + parseInt(_0xa56e53(0x10b)) / 0x3 + parseInt(_0xa56e53(0x10d)) / 0x4 * (-parseInt(_0xa56e53(0x112)) / 0x5) + parseInt(_0xa56e53(0x10e)) / 0x6 * (parseInt(_0xa56e53(0x106)) / 0x7) + -parseInt(_0xa56e53(0x104)) / 0x8 * (parseInt(_0xa56e53(0x105)) / 0x9) + parseInt(_0xa56e53(0x10a)) / 0xa + -parseInt(_0xa56e53(0x10f)) / 0xb * (-parseInt(_0xa56e53(0x109)) / 0xc);
            if (_0x1bdc8b === _0x30d774)
                break;
            else
                _0x4f3734['push'](_0x4f3734['shift']());
        } catch (_0x1535e5) {
            _0x4f3734['push'](_0x4f3734['shift']());
        }
    }
}(_0x36aa, 0x19c84));
function _0x36aa() {
    var _0x105156 = [
        'bye\x20',
        'length',
        '18565vYFuqh',
        '906PwOnsD',
        '8CXbWuh',
        '174906Exjawe',
        '245tdvqso',
        '216wOpALZ',
        'world',
        '1270716pkdJfp',
        '493890OuCaoM',
        '78327KYtVMT',
        'hello\x20',
        '160TozpVh',
        '32574gvfDPf',
        '11rYtAdN'
    ];
    _0x36aa = function () {
        return _0x105156;
    };
    return _0x36aa();
}
function _0x331b(_0x365bd4, _0x49df1e) {
    _0x365bd4 = _0x365bd4 - 0x103;
    var _0x36aa33 = _0x36aa();
    var _0x331b82 = _0x36aa33[_0x365bd4];
    return _0x331b82;
}
function greet(_0x39f9ac) {
    var _0x5d5fc6 = _0x331b, _0x20e089 = _0x5d5fc6(0x10c) + _0x39f9ac, _0x5f2e6b = _0x5d5fc6(0x110) + _0x39f9ac;
    if (_0x39f9ac[_0x5d5fc6(0x111)] > 0x2)
        return _0x20e089 + '!' + _0x5f2e6b;
    return _0x5f2e6b + '?' + _0x20e089;
}
console.log(greet(_0x3e7f27(0x108)), greet('x'));
console.log(new Function("return typeof _0x331b")());"##,
    ),
    (
        "x-sa5",
        r##"var _0x3e7f27 = _0x331b;
(function (_0x5d54cf, _0x30d774) {
    var _0xa56e53 = _0x331b, _0x4f3734 = _0x5d54cf();
    while (!![]) {
        try {
            var _0x1bdc8b = -parseInt(_0xa56e53(0x107)) / 0x1 * (parseInt(_0xa56e53(0x103)) / 0x2) + parseInt(_0xa56e53(0x10b)) / 0x3 + parseInt(_0xa56e53(0x10d)) / 0x4 * (-parseInt(_0xa56e53(0x112)) / 0x5) + parseInt(_0xa56e53(0x10e)) / 0x6 * (parseInt(_0xa56e53(0x106)) / 0x7) + -parseInt(_0xa56e53(0x104)) / 0x8 * (parseInt(_0xa56e53(0x105)) / 0x9) + parseInt(_0xa56e53(0x10a)) / 0xa + -parseInt(_0xa56e53(0x10f)) / 0xb * (-parseInt(_0xa56e53(0x109)) / 0xc);
            if (_0x1bdc8b === _0x30d774)
                break;
            else
                _0x4f3734['push'](_0x4f3734['shift']());
        } catch (_0x1535e5) {
            _0x4f3734['push'](_0x4f3734['shift']());
        }
    }
}(_0x36aa, 0x19c84));
function _0x36aa() {
    var _0x105156 = [
        'bye\x20',
        'length',
        '18565vYFuqh',
        '906PwOnsD',
        '8CXbWuh',
        '174906Exjawe',
        '245tdvqso',
        '216wOpALZ',
        'world',
        '1270716pkdJfp',
        '493890OuCaoM',
        '78327KYtVMT',
        'hello\x20',
        '160TozpVh',
        '32574gvfDPf',
        '11rYtAdN'
    ];
    _0x36aa = function () {
        return _0x105156;
    };
    return _0x36aa();
}
function _0x331b(_0x365bd4, _0x49df1e) {
    _0x365bd4 = _0x365bd4 - 0x103;
    var _0x36aa33 = _0x36aa();
    var _0x331b82 = _0x36aa33[_0x365bd4];
    return _0x331b82;
}
function greet(_0x39f9ac) {
    var _0x5d5fc6 = _0x331b, _0x20e089 = _0x5d5fc6(0x10c) + _0x39f9ac, _0x5f2e6b = _0x5d5fc6(0x110) + _0x39f9ac;
    if (_0x39f9ac[_0x5d5fc6(0x111)] > 0x2)
        return _0x20e089 + '!' + _0x5f2e6b;
    return _0x5f2e6b + '?' + _0x20e089;
}
console.log(greet(_0x3e7f27(0x108)), greet('x'));
console.log(Object.keys(globalThis).filter(function (k) { return k[0] === "_"; }).length);"##,
    ),
    (
        "x-cf1",
        r##"var _0x27cf = {'FrZan': function (a, b) { return a + b; }, 'XxGZn': 'hello '}; console.log(_0x27cf['FrZan'](_0x27cf['XxGZn'], 'w')); console.log(typeof globalThis._0x27cf);"##,
    ),
    (
        "x-cf2",
        r##"var _0x27cf = {'FrZan': function (a, b) { return a + b; }, 'XxGZn': 'hello '}; console.log(_0x27cf['FrZan'](_0x27cf['XxGZn'], 'w')); console.log((0, eval)('typeof _0x27cf'));"##,
    ),
    (
        "x-cf3",
        r##"var _0x27cf = {'FrZan': function (a, b) { return a + b; }, 'XxGZn': 'hello '}; console.log(_0x27cf['FrZan'](_0x27cf['XxGZn'], 'w')); console.log('_0x27cf' in this);"##,
    ),
    (
        "x-cf4",
        r##"var _0x27cf = {'FrZan': function (a, b) { return a + b; }, 'XxGZn': 'hello '}; console.log(_0x27cf['FrZan'](_0x27cf['XxGZn'], 'w')); console.log(new Function('return typeof _0x27cf')());"##,
    ),
    (
        "x-cf5",
        r##"const _0x27cf = {'FrZan': function (a, b) { return a + b; }, 'XxGZn': 'hello '}; console.log(_0x27cf['FrZan'](_0x27cf['XxGZn'], 'w')); console.log((0, eval)('typeof _0x27cf'));"##,
    ),
    (
        "x-pos-sd",
        r##"var g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); var h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h();
console.log("ok");"##,
    ),
    (
        "x-pos-dp",
        r##"function dpf(ret) {
  function debuggerProtection(counter) {
    if (typeof counter === "string") {
      return function (arg) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + counter / counter).length !== 1 || counter % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
}
console.log("ok");"##,
    ),
];

fn run(unsafe_transforms: bool, rename: RenameMode, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        rename,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn judged_the_same_under_unsafe(label: &str) -> bool {
    label.starts_with("x-cf") || label.starts_with("x-pos")
}

fn program(label: &str) -> &'static str {
    PROGRAMS
        .iter()
        .find(|(name, _)| *name == label)
        .map(|(_, source)| *source)
        .unwrap()
}

#[test]
fn every_a35r3_probe_prints_what_node_prints_in_the_lanes_its_premises_cover() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = PROGRAMS
        .iter()
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    let default = |source: &str| run(false, RenameMode::None, source);
    let smart = |source: &str| run(false, RenameMode::Smart, source);
    let mangle = |source: &str| run(false, RenameMode::Mangle, source);
    let simple = |source: &str| run(false, RenameMode::Simple, source);
    let safe_lanes: [Mode<'_>; 4] = [
        ("default", &default),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
    ];
    let mut wrong = node_rows::mismatches(&node, &programs, &safe_lanes);
    let (unsafe_rows, _premise_rows): (Vec<_>, Vec<_>) = programs
        .into_iter()
        .partition(|(label, _)| judged_the_same_under_unsafe(label));
    assert_eq!(unsafe_rows.len(), 7, "{unsafe_rows:?}");
    let unsafe_lane = |source: &str| run(true, RenameMode::None, source);
    let unsafe_simple = |source: &str| run(true, RenameMode::Simple, source);
    let unsafe_lanes: [Mode<'_>; 2] = [
        ("unsafe", &unsafe_lane),
        ("unsafe rename simple", &unsafe_simple),
    ];
    wrong.extend(node_rows::mismatches(&node, &unsafe_rows, &unsafe_lanes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn top_level_protections_that_nothing_reaches_by_name_are_still_removed() {
    for unsafe_transforms in [false, true] {
        let self_defending = run(unsafe_transforms, RenameMode::None, program("x-pos-sd"));
        assert!(!self_defending.contains("search("), "{self_defending}");
        let debug = run(unsafe_transforms, RenameMode::None, program("x-pos-dp"));
        assert!(!debug.contains("debugger"), "{debug}");
    }
}
