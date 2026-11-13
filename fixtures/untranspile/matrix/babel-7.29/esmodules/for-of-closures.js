"use strict";

function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
var fns = [];
var _loop = function _loop() {
  var doubled = v * 2;
  var note = "n" + v;
  fns.push(() => doubled + note);
  note += "!";
};
for (var v of new Set([1, 2, 3])) {
  _loop();
}
console.log(fns.map(f => f()).join(","));
var seen = [];
var _loop2 = function _loop2() {
    _ref2 = _slicedToArray(_ref3, 2);
    var key = _ref2[0];
    var value = _ref2[1];
    seen.push(() => key + value);
  },
  _ref2;
for (var _ref3 of new Map([["a", 1], ["b", 2]])) {
  _loop2();
}
console.log(seen.map(f => f()).join(","));
var later = [];
var _loop3 = function _loop3(word) {
  var count = 0;
  later.push(() => word + ++count);
};
for (var word of "abc") {
  _loop3(word);
}
console.log(later.map(f => f() + f()).join(","));
