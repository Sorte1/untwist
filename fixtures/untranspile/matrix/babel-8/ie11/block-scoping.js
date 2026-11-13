function _createForOfIteratorHelper(e, t) { var _n = "undefined" != typeof Symbol && e[Symbol.iterator] || e["@@iterator"]; if (!_n) { if (Array.isArray(e) || (_n = _unsupportedIterableToArray(e)) || t && e && "number" == typeof e.length) { _n && (e = _n); var o = 0, F = function F() {}; return { s: F, n: function n() { return o >= e.length ? { done: !0 } : { done: !1, value: e[o++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var a, u, i = !0; return { s: function s() { _n = _n.call(e); }, n: function n() { i = !0; var r = _n.next(); return r.done ? { done: !0 } : { value: r.value, done: i = !1 }; }, e: function e(r) { a = !0, u = r; }, f: function f() { try { i || null == _n.return || _n.return(); } finally { if (a) throw u; } } }; }
function _unsupportedIterableToArray(t, e) { if (t) { if ("string" == typeof t) return _arrayLikeToArray(t, e); var o = {}.toString.call(t).slice(8, -1); return "Object" === o && t.constructor && (o = t.constructor.name), "Map" === o || "Set" === o ? Array.from(t) : "Arguments" === o || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(o) ? _arrayLikeToArray(t, e) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
var fns = [];
var _loop = function _loop(i) {
  fns.push(function () {
    return i;
  });
};
for (var i = 0; i < 3; i++) {
  _loop(i);
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var later = [];
var _loop2 = function _loop2(_i) {
    if (_i === 1) return 0; // continue
    if (_i === 3) return 1; // break
    later.push(function () {
      return _i * 10;
    });
  },
  _ret;
for (var _i = 0; _i < 4; _i++) {
  _ret = _loop2(_i);
  if (_ret === 0) continue;
  if (_ret === 1) break;
}
console.log(later.map(function (f) {
  return f();
}).join(","));
function find(list) {
  var _iterator = _createForOfIteratorHelper(list),
    _step;
  try {
    var _loop3 = function _loop3() {
        var item = _step.value;
        var handler = function handler() {
          return item.id;
        };
        if (item.ok) return {
          v: handler()
        };
      },
      _ret2;
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      _ret2 = _loop3();
      if (_ret2) return _ret2.v;
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  return -1;
}
console.log(find([{
  id: 1
}, {
  id: 2,
  ok: true
}]));
var x = "outer";
{
  var _x = "inner";
  var y = _x + "!";
  console.log(_x, y);
}
console.log(x);
var ws = [];
var j = 0;
var _loop4 = function _loop4() {
  var k = j;
  ws.push(function () {
    return k;
  });
  j++;
};
while (j < 3) {
  _loop4();
}
console.log(ws.map(function (f) {
  return f();
}).join(","));
var _loop5 = function _loop5(_i2) {
  setTimeout(function () {
    return console.log("timer", _i2);
  }, 0);
};
for (var _i2 = 0, n = 2; _i2 < n; _i2++) {
  _loop5(_i2);
}
switch (1) {
  case 1:
    {
      var s = "case";
      console.log(s);
    }
}
