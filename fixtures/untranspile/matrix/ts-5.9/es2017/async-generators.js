"use strict";
var __await = (this && this.__await) || function (v) { return this instanceof __await ? (this.v = v, this) : new __await(v); }
var __asyncGenerator = (this && this.__asyncGenerator) || function (thisArg, _arguments, generator) {
    if (!Symbol.asyncIterator) throw new TypeError("Symbol.asyncIterator is not defined.");
    var g = generator.apply(thisArg, _arguments || []), i, q = [];
    return i = Object.create((typeof AsyncIterator === "function" ? AsyncIterator : Object).prototype), verb("next"), verb("throw"), verb("return", awaitReturn), i[Symbol.asyncIterator] = function () { return this; }, i;
    function awaitReturn(f) { return function (v) { return Promise.resolve(v).then(f, reject); }; }
    function verb(n, f) { if (g[n]) { i[n] = function (v) { return new Promise(function (a, b) { q.push([n, v, a, b]) > 1 || resume(n, v); }); }; if (f) i[n] = f(i[n]); } }
    function resume(n, v) { try { step(g[n](v)); } catch (e) { settle(q[0][3], e); } }
    function step(r) { r.value instanceof __await ? Promise.resolve(r.value.v).then(fulfill, reject) : settle(q[0][2], r); }
    function fulfill(value) { resume("next", value); }
    function reject(value) { resume("throw", value); }
    function settle(f, v) { if (f(v), q.shift(), q.length) resume(q[0][0], q[0][1]); }
};
var __asyncValues = (this && this.__asyncValues) || function (o) {
    if (!Symbol.asyncIterator) throw new TypeError("Symbol.asyncIterator is not defined.");
    var m = o[Symbol.asyncIterator], i;
    return m ? m.call(o) : (o = typeof __values === "function" ? __values(o) : o[Symbol.iterator](), i = {}, verb("next"), verb("throw"), verb("return"), i[Symbol.asyncIterator] = function () { return this; }, i);
    function verb(n) { i[n] = o[n] && function (v) { return new Promise(function (resolve, reject) { v = o[n](v), settle(resolve, reject, v.done, v.value); }); }; }
    function settle(resolve, reject, d, v) { Promise.resolve(v).then(function(v) { resolve({ value: v, done: d }); }, reject); }
};
function ticker(n) {
    return __asyncGenerator(this, arguments, function* ticker_1() {
        for (let i = 0; i < n; i++) {
            yield __await(null);
            yield yield __await(i * 10);
        }
    });
}
function relay(src) {
    return __asyncGenerator(this, arguments, function* relay_1() {
        var _a, e_1, _b, _c;
        try {
            for (var _d = true, src_1 = __asyncValues(src), src_1_1; src_1_1 = yield __await(src_1.next()), _a = src_1_1.done, !_a; _d = true) {
                _c = src_1_1.value;
                _d = false;
                const v = _c;
                yield yield __await(v + 1);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (!_d && !_a && (_b = src_1.return)) yield __await(_b.call(src_1));
            }
            finally { if (e_1) throw e_1.error; }
        }
    });
}
async function main() {
    var _a, e_2, _b, _c, _d, e_3, _e, _f;
    const seen = [];
    try {
        for (var _g = true, _h = __asyncValues(relay(ticker(3))), _j; _j = await _h.next(), _a = _j.done, !_a; _g = true) {
            _c = _j.value;
            _g = false;
            const v = _c;
            seen.push(v);
        }
    }
    catch (e_2_1) { e_2 = { error: e_2_1 }; }
    finally {
        try {
            if (!_g && !_a && (_b = _h.return)) await _b.call(_h);
        }
        finally { if (e_2) throw e_2.error; }
    }
    console.log(seen.join(","));
    const it = ticker(5);
    console.log(JSON.stringify([await it.next(), await it.return("x"), await it.next()]));
    const mixed = [];
    try {
        for (var _k = true, _l = __asyncValues([Promise.resolve("a"), "b", later("c")]), _m; _m = await _l.next(), _d = _m.done, !_d; _k = true) {
            _f = _m.value;
            _k = false;
            const v = _f;
            mixed.push(v);
        }
    }
    catch (e_3_1) { e_3 = { error: e_3_1 }; }
    finally {
        try {
            if (!_k && !_d && (_e = _l.return)) await _e.call(_l);
        }
        finally { if (e_3) throw e_3.error; }
    }
    console.log(mixed.join(""));
}
function later(v) {
    return new Promise((resolve) => setTimeout(() => resolve(v), 0));
}
main();
