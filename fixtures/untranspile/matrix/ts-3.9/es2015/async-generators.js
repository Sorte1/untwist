"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __await = (this && this.__await) || function (v) { return this instanceof __await ? (this.v = v, this) : new __await(v); }
var __asyncGenerator = (this && this.__asyncGenerator) || function (thisArg, _arguments, generator) {
    if (!Symbol.asyncIterator) throw new TypeError("Symbol.asyncIterator is not defined.");
    var g = generator.apply(thisArg, _arguments || []), i, q = [];
    return i = {}, verb("next"), verb("throw"), verb("return"), i[Symbol.asyncIterator] = function () { return this; }, i;
    function verb(n) { if (g[n]) i[n] = function (v) { return new Promise(function (a, b) { q.push([n, v, a, b]) > 1 || resume(n, v); }); }; }
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
        var e_1, _a;
        try {
            for (var src_1 = __asyncValues(src), src_1_1; src_1_1 = yield __await(src_1.next()), !src_1_1.done;) {
                const v = src_1_1.value;
                yield yield __await(v + 1);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (src_1_1 && !src_1_1.done && (_a = src_1.return)) yield __await(_a.call(src_1));
            }
            finally { if (e_1) throw e_1.error; }
        }
    });
}
function main() {
    var e_2, _a, e_3, _b;
    return __awaiter(this, void 0, void 0, function* () {
        const seen = [];
        try {
            for (var _c = __asyncValues(relay(ticker(3))), _d; _d = yield _c.next(), !_d.done;) {
                const v = _d.value;
                seen.push(v);
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (_d && !_d.done && (_a = _c.return)) yield _a.call(_c);
            }
            finally { if (e_2) throw e_2.error; }
        }
        console.log(seen.join(","));
        const it = ticker(5);
        console.log(JSON.stringify([yield it.next(), yield it.return("x"), yield it.next()]));
        const mixed = [];
        try {
            for (var _e = __asyncValues([Promise.resolve("a"), "b", later("c")]), _f; _f = yield _e.next(), !_f.done;) {
                const v = _f.value;
                mixed.push(v);
            }
        }
        catch (e_3_1) { e_3 = { error: e_3_1 }; }
        finally {
            try {
                if (_f && !_f.done && (_b = _e.return)) yield _b.call(_e);
            }
            finally { if (e_3) throw e_3.error; }
        }
        console.log(mixed.join(""));
    });
}
function later(v) {
    return new Promise((resolve) => setTimeout(() => resolve(v), 0));
}
main();
