(function () {
  'use strict';

  /******************************************************************************
  Copyright (c) Microsoft Corporation.

  Permission to use, copy, modify, and/or distribute this software for any
  purpose with or without fee is hereby granted.

  THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
  REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
  AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
  INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
  LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
  OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
  PERFORMANCE OF THIS SOFTWARE.
  ***************************************************************************** */
  /* global Reflect, Promise, SuppressedError, Symbol, Iterator */


  function __generator(thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g = Object.create((typeof Iterator === "function" ? Iterator : Object).prototype);
    return g.next = verb(0), g["throw"] = verb(1), g["return"] = verb(2), typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
  }

  function __values(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
  }

  function __read(o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
  }

  function __spreadArray(to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
  }

  typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
  };

  var e_1, _a, e_2, _b, e_3, _c, e_4, _d, e_5, _e, e_6, _f, e_7, _g, e_8, _h, e_9, _j, e_10, _k, e_11, _l;
  var log = [];
  function counted(name, items) {
      var _a;
      return _a = {},
          _a[Symbol.iterator] = function () {
              var at = 0;
              log.push(name + ":open");
              return {
                  next: function () {
                      log.push(name + ":next");
                      return at < items.length ? { value: items[at++], done: false } : { value: undefined, done: true };
                  },
                  return: function () {
                      log.push(name + ":return");
                      return { done: true };
                  },
              };
          },
          _a;
  }
  try {
      for (var _m = __values(counted("all", [1, 2])), _o = _m.next(); !_o.done; _o = _m.next()) {
          var v = _o.value;
          log.push("all=" + v);
      }
  }
  catch (e_1_1) { e_1 = { error: e_1_1 }; }
  finally {
      try {
          if (_o && !_o.done && (_a = _m.return)) _a.call(_m);
      }
      finally { if (e_1) throw e_1.error; }
  }
  try {
      for (var _p = __values(counted("brk", [1, 2, 3])), _q = _p.next(); !_q.done; _q = _p.next()) {
          var v = _q.value;
          if (v === 2)
              break;
          log.push("brk=" + v);
      }
  }
  catch (e_2_1) { e_2 = { error: e_2_1 }; }
  finally {
      try {
          if (_q && !_q.done && (_b = _p.return)) _b.call(_p);
      }
      finally { if (e_2) throw e_2.error; }
  }
  function early() {
      var e_12, _a;
      try {
          for (var _b = __values(counted("ret", [5, 6, 7])), _c = _b.next(); !_c.done; _c = _b.next()) {
              var v = _c.value;
              if (v === 6)
                  return v;
          }
      }
      catch (e_12_1) { e_12 = { error: e_12_1 }; }
      finally {
          try {
              if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
          }
          finally { if (e_12) throw e_12.error; }
      }
      return -1;
  }
  log.push("early=" + early());
  try {
      try {
          for (var _r = __values(counted("thr", [1, 2])), _s = _r.next(); !_s.done; _s = _r.next()) {
              var v = _s.value;
              if (v === 2)
                  throw new Error("boom " + v);
          }
      }
      catch (e_3_1) { e_3 = { error: e_3_1 }; }
      finally {
          try {
              if (_s && !_s.done && (_c = _r.return)) _c.call(_r);
          }
          finally { if (e_3) throw e_3.error; }
      }
  }
  catch (err) {
      log.push("caught " + err.message);
  }
  var skipped = 0;
  try {
      for (var _t = __values(counted("cont", [1, 2, 3])), _u = _t.next(); !_u.done; _u = _t.next()) {
          var v = _u.value;
          if (v === 2)
              continue;
          skipped += v;
      }
  }
  catch (e_4_1) { e_4 = { error: e_4_1 }; }
  finally {
      try {
          if (_u && !_u.done && (_d = _t.return)) _d.call(_t);
      }
      finally { if (e_4) throw e_4.error; }
  }
  log.push("cont=" + skipped);
  try {
      outer: for (var _v = __values(counted("outer", [1, 2])), _w = _v.next(); !_w.done; _w = _v.next()) {
          var a = _w.value;
          try {
              for (var _x = (e_6 = void 0, __values(counted("inner", [10, 20]))), _y = _x.next(); !_y.done; _y = _x.next()) {
                  var b = _y.value;
                  if (b === 20)
                      continue outer;
                  log.push(a + b);
              }
          }
          catch (e_6_1) { e_6 = { error: e_6_1 }; }
          finally {
              try {
                  if (_y && !_y.done && (_f = _x.return)) _f.call(_x);
              }
              finally { if (e_6) throw e_6.error; }
          }
      }
  }
  catch (e_5_1) { e_5 = { error: e_5_1 }; }
  finally {
      try {
          if (_w && !_w.done && (_e = _v.return)) _e.call(_v);
      }
      finally { if (e_5) throw e_5.error; }
  }
  var last;
  try {
      for (var _z = __values(counted("assign", [7, 8])), _0 = _z.next(); !_0.done; _0 = _z.next()) {
          last = _0.value;
          log.push("seen " + last);
      }
  }
  catch (e_7_1) { e_7 = { error: e_7_1 }; }
  finally {
      try {
          if (_0 && !_0.done && (_g = _z.return)) _g.call(_z);
      }
      finally { if (e_7) throw e_7.error; }
  }
  log.push("last=" + last);
  var pairs = new Map([["x", 1], ["y", 2]]);
  try {
      for (var pairs_1 = __values(pairs), pairs_1_1 = pairs_1.next(); !pairs_1_1.done; pairs_1_1 = pairs_1.next()) {
          var _1 = __read(pairs_1_1.value, 2), key = _1[0], value = _1[1];
          log.push(key + value);
      }
  }
  catch (e_8_1) { e_8 = { error: e_8_1 }; }
  finally {
      try {
          if (pairs_1_1 && !pairs_1_1.done && (_h = pairs_1.return)) _h.call(pairs_1);
      }
      finally { if (e_8) throw e_8.error; }
  }
  var fns = [];
  var _loop_1 = function (v) {
      fns.push(function () { return v; });
  };
  try {
      for (var _2 = __values([1, 2, 3]), _3 = _2.next(); !_3.done; _3 = _2.next()) {
          var v = _3.value;
          _loop_1(v);
      }
  }
  catch (e_9_1) { e_9 = { error: e_9_1 }; }
  finally {
      try {
          if (_3 && !_3.done && (_j = _2.return)) _j.call(_2);
      }
      finally { if (e_9) throw e_9.error; }
  }
  log.push(fns.map(function (f) { return f(); }).join(""));
  function gen() {
      return __generator(this, function (_a) {
          switch (_a.label) {
              case 0:
                  _a.trys.push([0, , 3, 4]);
                  return [4 /*yield*/, 1];
              case 1:
                  _a.sent();
                  return [4 /*yield*/, 2];
              case 2:
                  _a.sent();
                  return [3 /*break*/, 4];
              case 3:
                  log.push("gen:finally");
                  return [7 /*endfinally*/];
              case 4: return [2 /*return*/];
          }
      });
  }
  try {
      for (var _4 = __values(gen()), _5 = _4.next(); !_5.done; _5 = _4.next()) {
          var v = _5.value;
          if (v === 1)
              break;
      }
  }
  catch (e_10_1) { e_10 = { error: e_10_1 }; }
  finally {
      try {
          if (_5 && !_5.done && (_k = _4.return)) _k.call(_4);
      }
      finally { if (e_10) throw e_10.error; }
  }
  try {
      try {
          for (var _6 = __values(counted("bad", [1])), _7 = _6.next(); !_7.done; _7 = _6.next()) {
              var v = _7.value;
              log.push(v.missing.field);
          }
      }
      catch (e_11_1) { e_11 = { error: e_11_1 }; }
      finally {
          try {
              if (_7 && !_7.done && (_l = _6.return)) _l.call(_6);
          }
          finally { if (e_11) throw e_11.error; }
      }
  }
  catch (err) {
      log.push(err.constructor.name);
  }
  var spreadSet = __spreadArray([], __read(new Set([4, 4, 5])), false);
  var _8 = __read(new Map([["k", 1]]).keys(), 1), firstKey = _8[0];
  log.push(spreadSet.join("") + firstKey);
  console.log(log.join("\n"));

})();
