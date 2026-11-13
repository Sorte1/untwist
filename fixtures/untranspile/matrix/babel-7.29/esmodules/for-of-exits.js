"use strict";

function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
var log = [];
function counted(name, items) {
  return {
    [Symbol.iterator]() {
      var at = 0;
      log.push(name + ":open");
      return {
        next() {
          log.push(name + ":next");
          return at < items.length ? {
            value: items[at++],
            done: false
          } : {
            value: undefined,
            done: true
          };
        },
        return() {
          log.push(name + ":return");
          return {
            done: true
          };
        }
      };
    }
  };
}
for (var v of counted("all", [1, 2])) log.push("all=" + v);
for (var _v of counted("brk", [1, 2, 3])) {
  if (_v === 2) break;
  log.push("brk=" + _v);
}
function early() {
  for (var _v2 of counted("ret", [5, 6, 7])) {
    if (_v2 === 6) return _v2;
  }
  return -1;
}
log.push("early=" + early());
try {
  for (var _v3 of counted("thr", [1, 2])) {
    if (_v3 === 2) throw new Error("boom " + _v3);
  }
} catch (err) {
  log.push("caught " + err.message);
}
var skipped = 0;
for (var _v4 of counted("cont", [1, 2, 3])) {
  if (_v4 === 2) continue;
  skipped += _v4;
}
log.push("cont=" + skipped);
outer: for (var a of counted("outer", [1, 2])) {
  for (var b of counted("inner", [10, 20])) {
    if (b === 20) continue outer;
    log.push(a + b);
  }
}
var last;
for (last of counted("assign", [7, 8])) log.push("seen " + last);
log.push("last=" + last);
var pairs = new Map([["x", 1], ["y", 2]]);
for (var _ref3 of pairs) {
  var _ref2 = _slicedToArray(_ref3, 2);
  var key = _ref2[0];
  var value = _ref2[1];
  log.push(key + value);
}
var fns = [];
var _loop = function _loop(_v5) {
  fns.push(() => _v5);
};
for (var _v5 of [1, 2, 3]) {
  _loop(_v5);
}
log.push(fns.map(f => f()).join(""));
function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    log.push("gen:finally");
  }
}
for (var _v6 of gen()) {
  if (_v6 === 1) break;
}
try {
  for (var _v7 of counted("bad", [1])) {
    log.push(_v7.missing.field);
  }
} catch (err) {
  log.push(err.constructor.name);
}
var spreadSet = [...new Set([4, 4, 5])];
var _Map$keys = new Map([["k", 1]]).keys(),
  _Map$keys2 = _slicedToArray(_Map$keys, 1),
  firstKey = _Map$keys2[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
