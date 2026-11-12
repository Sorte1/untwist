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
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
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
    return __asyncGenerator(this, arguments, function ticker_1() {
        var i;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    i = 0;
                    _a.label = 1;
                case 1:
                    if (!(i < n)) return [3 /*break*/, 6];
                    return [4 /*yield*/, __await(null)];
                case 2:
                    _a.sent();
                    return [4 /*yield*/, __await(i * 10)];
                case 3: return [4 /*yield*/, _a.sent()];
                case 4:
                    _a.sent();
                    _a.label = 5;
                case 5:
                    i++;
                    return [3 /*break*/, 1];
                case 6: return [2 /*return*/];
            }
        });
    });
}
function relay(src) {
    return __asyncGenerator(this, arguments, function relay_1() {
        var _a, src_1, src_1_1, v, e_1_1;
        var _b, e_1, _c, _d;
        return __generator(this, function (_e) {
            switch (_e.label) {
                case 0:
                    _e.trys.push([0, 9, 10, 15]);
                    _a = true, src_1 = __asyncValues(src);
                    _e.label = 1;
                case 1: return [4 /*yield*/, __await(src_1.next())];
                case 2:
                    if (!(src_1_1 = _e.sent(), _b = src_1_1.done, !_b)) return [3 /*break*/, 8];
                    _d = src_1_1.value;
                    _a = false;
                    _e.label = 3;
                case 3:
                    _e.trys.push([3, , 6, 7]);
                    v = _d;
                    return [4 /*yield*/, __await(v + 1)];
                case 4: return [4 /*yield*/, _e.sent()];
                case 5:
                    _e.sent();
                    return [3 /*break*/, 7];
                case 6:
                    _a = true;
                    return [7 /*endfinally*/];
                case 7: return [3 /*break*/, 1];
                case 8: return [3 /*break*/, 15];
                case 9:
                    e_1_1 = _e.sent();
                    e_1 = { error: e_1_1 };
                    return [3 /*break*/, 15];
                case 10:
                    _e.trys.push([10, , 13, 14]);
                    if (!(!_a && !_b && (_c = src_1["return"]))) return [3 /*break*/, 12];
                    return [4 /*yield*/, __await(_c.call(src_1))];
                case 11:
                    _e.sent();
                    _e.label = 12;
                case 12: return [3 /*break*/, 14];
                case 13:
                    if (e_1) throw e_1.error;
                    return [7 /*endfinally*/];
                case 14: return [7 /*endfinally*/];
                case 15: return [2 /*return*/];
            }
        });
    });
}
function main() {
    var _a, e_2, _b, _c, _d, e_3, _e, _f;
    return __awaiter(this, void 0, void 0, function () {
        var seen, _g, _h, _j, v, e_2_1, it, _k, _l, _m, _o, _p, mixed, _q, _r, _s, v, e_3_1;
        return __generator(this, function (_t) {
            switch (_t.label) {
                case 0:
                    seen = [];
                    _t.label = 1;
                case 1:
                    _t.trys.push([1, 6, 7, 12]);
                    _g = true, _h = __asyncValues(relay(ticker(3)));
                    _t.label = 2;
                case 2: return [4 /*yield*/, _h.next()];
                case 3:
                    if (!(_j = _t.sent(), _a = _j.done, !_a)) return [3 /*break*/, 5];
                    _c = _j.value;
                    _g = false;
                    try {
                        v = _c;
                        seen.push(v);
                    }
                    finally {
                        _g = true;
                    }
                    _t.label = 4;
                case 4: return [3 /*break*/, 2];
                case 5: return [3 /*break*/, 12];
                case 6:
                    e_2_1 = _t.sent();
                    e_2 = { error: e_2_1 };
                    return [3 /*break*/, 12];
                case 7:
                    _t.trys.push([7, , 10, 11]);
                    if (!(!_g && !_a && (_b = _h["return"]))) return [3 /*break*/, 9];
                    return [4 /*yield*/, _b.call(_h)];
                case 8:
                    _t.sent();
                    _t.label = 9;
                case 9: return [3 /*break*/, 11];
                case 10:
                    if (e_2) throw e_2.error;
                    return [7 /*endfinally*/];
                case 11: return [7 /*endfinally*/];
                case 12:
                    console.log(seen.join(","));
                    it = ticker(5);
                    _l = (_k = console).log;
                    _o = (_m = JSON).stringify;
                    return [4 /*yield*/, it.next()];
                case 13:
                    _p = [_t.sent()];
                    return [4 /*yield*/, it["return"]("x")];
                case 14:
                    _p = _p.concat([_t.sent()]);
                    return [4 /*yield*/, it.next()];
                case 15:
                    _l.apply(_k, [_o.apply(_m, [_p.concat([_t.sent()])])]);
                    mixed = [];
                    _t.label = 16;
                case 16:
                    _t.trys.push([16, 21, 22, 27]);
                    _q = true, _r = __asyncValues([Promise.resolve("a"), "b", later("c")]);
                    _t.label = 17;
                case 17: return [4 /*yield*/, _r.next()];
                case 18:
                    if (!(_s = _t.sent(), _d = _s.done, !_d)) return [3 /*break*/, 20];
                    _f = _s.value;
                    _q = false;
                    try {
                        v = _f;
                        mixed.push(v);
                    }
                    finally {
                        _q = true;
                    }
                    _t.label = 19;
                case 19: return [3 /*break*/, 17];
                case 20: return [3 /*break*/, 27];
                case 21:
                    e_3_1 = _t.sent();
                    e_3 = { error: e_3_1 };
                    return [3 /*break*/, 27];
                case 22:
                    _t.trys.push([22, , 25, 26]);
                    if (!(!_q && !_d && (_e = _r["return"]))) return [3 /*break*/, 24];
                    return [4 /*yield*/, _e.call(_r)];
                case 23:
                    _t.sent();
                    _t.label = 24;
                case 24: return [3 /*break*/, 26];
                case 25:
                    if (e_3) throw e_3.error;
                    return [7 /*endfinally*/];
                case 26: return [7 /*endfinally*/];
                case 27:
                    console.log(mixed.join(""));
                    return [2 /*return*/];
            }
        });
    });
}
function later(v) {
    return new Promise(function (resolve) { return setTimeout(function () { return resolve(v); }, 0); });
}
main();
