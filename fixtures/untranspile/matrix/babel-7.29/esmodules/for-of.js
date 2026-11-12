"use strict";

function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
var out = [];
for (var v of [1, 2, 3]) out.push(v * 2);
for (var ch of "héllo") out.push(ch);
for (var _ref3 of new Map([["a", 1], ["b", 2]])) {
  var _ref2 = _slicedToArray(_ref3, 2);
  var k = _ref2[0];
  var _v = _ref2[1];
  out.push(k + _v);
}
for (var _v2 of new Set([3, 3, 4])) out.push(_v2);
console.log(out.join(","));
var custom = {
  from: 1,
  to: 4,
  [Symbol.iterator]() {
    var cur = this.from;
    var last = this.to;
    return {
      next: () => cur <= last ? {
        value: cur++,
        done: false
      } : {
        value: undefined,
        done: true
      },
      return() {
        out.push("closed");
        return {
          done: true
        };
      }
    };
  }
};
var got = [];
for (var _v3 of custom) {
  if (_v3 === 3) break;
  got.push(_v3);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
  var xs = [];
  for (var a of arguments) xs.push(a);
  return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
outer: for (var row of [[1, 2], [3, 4], [5, 6]]) {
  for (var cell of row) {
    if (cell === 4) continue outer;
    if (cell === 6) break outer;
    total += cell;
  }
}
console.log(total);
