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

  /** @deprecated */
  function __spread() {
    for (var ar = [], i = 0; i < arguments.length; i++)
        ar = ar.concat(__read(arguments[i]));
    return ar;
  }

  typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
  };

  var _a, e_1, _b;
  function range(start, end, step) {
      var i;
      if (step === void 0) { step = 1; }
      return __generator(this, function (_a) {
          switch (_a.label) {
              case 0:
                  i = start;
                  _a.label = 1;
              case 1:
                  if (!(i < end)) return [3 /*break*/, 4];
                  return [4 /*yield*/, i];
              case 2:
                  _a.sent();
                  _a.label = 3;
              case 3:
                  i += step;
                  return [3 /*break*/, 1];
              case 4: return [2 /*return*/];
          }
      });
  }
  function inner() {
      var got;
      return __generator(this, function (_a) {
          switch (_a.label) {
              case 0: return [4 /*yield*/, "first"];
              case 1:
                  got = _a.sent();
                  return [4 /*yield*/, "got " + got];
              case 2:
                  _a.sent();
                  return [2 /*return*/, "inner-done"];
          }
      });
  }
  function outer() {
      var r;
      return __generator(this, function (_a) {
          switch (_a.label) {
              case 0: return [5 /*yield**/, __values(inner())];
              case 1:
                  r = _a.sent();
                  return [4 /*yield*/, r];
              case 2:
                  _a.sent();
                  _a.label = 3;
              case 3:
                  _a.trys.push([3, , 5, 7]);
                  return [4 /*yield*/, "try"];
              case 4:
                  _a.sent();
                  return [3 /*break*/, 7];
              case 5: return [4 /*yield*/, "cleanup"];
              case 6:
                  _a.sent();
                  return [7 /*endfinally*/];
              case 7: return [2 /*return*/];
          }
      });
  }
  console.log(__spread(range(0, 5)).join(","), Array.from(range(1, 10, 3)).join(","));
  var it = outer();
  console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
  var obj = (_a = {},
      _a[Symbol.iterator] = function () {
          return __generator(this, function (_a) {
              switch (_a.label) {
                  case 0: return [4 /*yield*/, 1];
                  case 1:
                      _a.sent();
                      return [4 /*yield*/, 2];
                  case 2:
                      _a.sent();
                      return [2 /*return*/];
              }
          });
      },
      _a);
  console.log(__spread(obj).join(","));
  function thrower() {
      var e_2;
      return __generator(this, function (_a) {
          switch (_a.label) {
              case 0:
                  _a.trys.push([0, 2, , 4]);
                  return [4 /*yield*/, 1];
              case 1:
                  _a.sent();
                  return [3 /*break*/, 4];
              case 2:
                  e_2 = _a.sent();
                  return [4 /*yield*/, "caught " + e_2];
              case 3:
                  _a.sent();
                  return [3 /*break*/, 4];
              case 4: return [2 /*return*/];
          }
      });
  }
  var t = thrower();
  t.next();
  console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
  function fib() {
      var _a, a, b;
      var _b;
      return __generator(this, function (_c) {
          switch (_c.label) {
              case 0:
                  _a = __read([0, 1], 2), a = _a[0], b = _a[1];
                  _c.label = 1;
              case 1:
                  return [4 /*yield*/, a];
              case 2:
                  _c.sent();
                  _b = __read([b, a + b], 2), a = _b[0], b = _b[1];
                  return [3 /*break*/, 1];
              case 3: return [2 /*return*/];
          }
      });
  }
  var f = fib();
  var first = [];
  try {
      for (var f_1 = __values(f), f_1_1 = f_1.next(); !f_1_1.done; f_1_1 = f_1.next()) {
          var v = f_1_1.value;
          if (v > 20)
              break;
          first.push(v);
      }
  }
  catch (e_1_1) { e_1 = { error: e_1_1 }; }
  finally {
      try {
          if (f_1_1 && !f_1_1.done && (_b = f_1.return)) _b.call(f_1);
      }
      finally { if (e_1) throw e_1.error; }
  }
  console.log(first.join(","));

})();
