var _show9, _made;
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Kind = {
  ALPHA: "alpha",
  BETA: "beta",
  GAMMA: 3
};
var key = "konst";
var num = 7;
function show(label, o) {
  var parts = Object.getOwnPropertyNames(o).map(function (name) {
    var d = Object.getOwnPropertyDescriptor(o, name);
    var v = typeof d.value === "function" ? "fn:" + d.value.name : JSON.stringify(d.value);
    return name + "=" + v + (d.enumerable ? "e" : "") + (d.writable ? "w" : "") + (d.configurable ? "c" : "");
  });
  console.log(label, Object.getPrototypeOf(o) === Object.prototype, parts.join(" "));
}
var log = [];
function note(v) {
  log.push(v);
  return v;
}
show("literal", _defineProperty(_defineProperty(_defineProperty({}, "a", 1), "b", 2), 3, "three"));
show("plain-first", _defineProperty(_defineProperty({
  x: 1
}, "y", note(2)), "z", 3));
show("const-key", _defineProperty(_defineProperty({}, key, 1), num, 2));
show("enum-key", _defineProperty(_defineProperty(_defineProperty({}, Kind.ALPHA, 1), Kind.BETA, note("b")), Kind.GAMMA, 3));
show("duplicate", _defineProperty(_defineProperty(_defineProperty({}, "d", 1), "e", 2), "d", 3));
show("proto-key", _defineProperty({}, "__proto__", 5));
show("names", _defineProperty(_defineProperty(_defineProperty(_defineProperty({}, "f", function f() {}), "g", function g() {
  return 1;
}), "h", /*#__PURE__*/_createClass(function h() {
  "use strict";

  _classCallCheck(this, h);
})), "i", function named() {}));
show("nested", _defineProperty({}, "outer", _defineProperty({}, "inner", 1)));
show("many", (_show9 = {}, _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_show9, "k0", 0), "k1", 1), "k2", 2), "k3", 3), "k4", 4), "k5", 5), "k6", 6), "k7", 7), "k8", 8), "k9", 9), _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_show9, "k10", 10), "k11", note(11)), "k12", 12), "k13", 13), "k14", 14), "k15", 15), "k16", 16), "k17", 17), "k18", 18), "k19", 19), _defineProperty(_defineProperty(_show9, "k20", 20), "k21", 21)));
function build(a, b) {
  var _ref;
  return _ref = {
    first: a
  }, _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_ref, Kind.ALPHA, 1), Kind.BETA, 2), "c2", 3), "c3", 4), "c4", 5), "c5", 6), "c6", 7), "c7", 8), "c8", 9), "c9", 10), _defineProperty(_defineProperty(_defineProperty(_ref, "c10", note(b)), "c11", 12), "c12", log.length);
}
show("built", build(1, 2));
var made = (_made = {}, _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_made, "m0", 0), "m1", 1), "m2", 2), "m3", 3), "m4", 4), "m5", 5), "m6", 6), "m7", 7), "m8", 8), "m9", 9), _defineProperty(_defineProperty(_made, "m10", 10), "m11", 11));
show("made", made);
function param(k, v) {
  return _defineProperty(_defineProperty({}, k, v), "p", 1);
}
show("param-key", param("q", 2));
show("computed-key", _defineProperty({}, key + "x", 1));
(function (_ref4) {
  var Local = {
    ONE: "one",
    TWO: "two",
    THREE: 3
  };
  var Open = {
    ONE: "one"
  };
  Open.ONE = {
    toString: function toString() {
      log.push("open");
      return "uno";
    }
  };
  var tag = "tag";
  var later = "later";
  show("local-enum", _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty({}, Local.ONE, 1), Local.TWO, note(2)), Local.THREE, 3), tag, 4), later, 5));
  show("open-enum", _defineProperty(_defineProperty({}, Open.ONE, note(1)), "two", 2));
  function pick() {
    var _ref3;
    return _ref3 = {}, _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_ref3, Local.ONE, 0), Local.TWO, 1), "l2", 2), "l3", 3), "l4", 4), "l5", 5), "l6", 6), "l7", 7), "l8", 8), "l9", 9), _defineProperty(_defineProperty(_ref3, "l10", note(10)), "l11", log.length);
  }
  show("pick", pick());
  var listed = [1, (_ref4 = {
    x: 0
  }, _defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_defineProperty(_ref4, tag, 1), "a1", 1), "a2", 2), "a3", 3), "a4", 4), "a5", 5), "a6", 6), "a7", 7), "a8", 8), "a9", 9), _defineProperty(_defineProperty(_ref4, "a10", 10), "a11", 11))];
  show("listed", listed[1]);
})();
console.log(log.join(","));
