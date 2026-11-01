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
function ticks(n) {
    return __asyncGenerator(this, arguments, function* ticks_1() {
        for (let i = 0; i < n; i++) {
            yield __await(null);
            yield yield __await(i);
        }
    });
}
async function main() {
    var _a, e_1, _b, _c, _d, e_2, _e, _f;
    const out = [];
    try {
        for (var _g = true, _h = __asyncValues(ticks(3)), _j; _j = await _h.next(), _a = _j.done, !_a; _g = true) {
            _c = _j.value;
            _g = false;
            const v = _c;
            out.push(v);
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (!_g && !_a && (_b = _h.return)) await _b.call(_h);
        }
        finally { if (e_1) throw e_1.error; }
    }
    console.log(out.join(","));
    const it = ticks(2);
    console.log(JSON.stringify(await it.next()), JSON.stringify(await it.return(9)), JSON.stringify(await it.next()));
    const fromSync = [];
    try {
        for (var _k = true, _l = __asyncValues([Promise.resolve("a"), "b"]), _m; _m = await _l.next(), _d = _m.done, !_d; _k = true) {
            _f = _m.value;
            _k = false;
            const v = _f;
            fromSync.push(v);
        }
    }
    catch (e_2_1) { e_2 = { error: e_2_1 }; }
    finally {
        try {
            if (!_k && !_d && (_e = _l.return)) await _e.call(_l);
        }
        finally { if (e_2) throw e_2.error; }
    }
    console.log(fromSync.join(""));
}
main();
