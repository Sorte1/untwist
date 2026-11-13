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
        while (_) try {
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
        var src_1, src_1_1, v, e_1_1;
        var e_1, _a;
        return __generator(this, function (_b) {
            switch (_b.label) {
                case 0:
                    _b.trys.push([0, 7, 8, 13]);
                    src_1 = __asyncValues(src);
                    _b.label = 1;
                case 1: return [4 /*yield*/, __await(src_1.next())];
                case 2:
                    if (!(src_1_1 = _b.sent(), !src_1_1.done)) return [3 /*break*/, 6];
                    v = src_1_1.value;
                    return [4 /*yield*/, __await(v + 1)];
                case 3: return [4 /*yield*/, _b.sent()];
                case 4:
                    _b.sent();
                    _b.label = 5;
                case 5: return [3 /*break*/, 1];
                case 6: return [3 /*break*/, 13];
                case 7:
                    e_1_1 = _b.sent();
                    e_1 = { error: e_1_1 };
                    return [3 /*break*/, 13];
                case 8:
                    _b.trys.push([8, , 11, 12]);
                    if (!(src_1_1 && !src_1_1.done && (_a = src_1.return))) return [3 /*break*/, 10];
                    return [4 /*yield*/, __await(_a.call(src_1))];
                case 9:
                    _b.sent();
                    _b.label = 10;
                case 10: return [3 /*break*/, 12];
                case 11:
                    if (e_1) throw e_1.error;
                    return [7 /*endfinally*/];
                case 12: return [7 /*endfinally*/];
                case 13: return [2 /*return*/];
            }
        });
    });
}
function main() {
    var e_2, _a, e_3, _b;
    return __awaiter(this, void 0, void 0, function () {
        var seen, _c, _d, v, e_2_1, it, _e, _f, _g, _h, _j, mixed, _k, _l, v, e_3_1;
        return __generator(this, function (_m) {
            switch (_m.label) {
                case 0:
                    seen = [];
                    _m.label = 1;
                case 1:
                    _m.trys.push([1, 6, 7, 12]);
                    _c = __asyncValues(relay(ticker(3)));
                    _m.label = 2;
                case 2: return [4 /*yield*/, _c.next()];
                case 3:
                    if (!(_d = _m.sent(), !_d.done)) return [3 /*break*/, 5];
                    v = _d.value;
                    seen.push(v);
                    _m.label = 4;
                case 4: return [3 /*break*/, 2];
                case 5: return [3 /*break*/, 12];
                case 6:
                    e_2_1 = _m.sent();
                    e_2 = { error: e_2_1 };
                    return [3 /*break*/, 12];
                case 7:
                    _m.trys.push([7, , 10, 11]);
                    if (!(_d && !_d.done && (_a = _c.return))) return [3 /*break*/, 9];
                    return [4 /*yield*/, _a.call(_c)];
                case 8:
                    _m.sent();
                    _m.label = 9;
                case 9: return [3 /*break*/, 11];
                case 10:
                    if (e_2) throw e_2.error;
                    return [7 /*endfinally*/];
                case 11: return [7 /*endfinally*/];
                case 12:
                    console.log(seen.join(","));
                    it = ticker(5);
                    _f = (_e = console).log;
                    _h = (_g = JSON).stringify;
                    return [4 /*yield*/, it.next()];
                case 13:
                    _j = [_m.sent()];
                    return [4 /*yield*/, it.return("x")];
                case 14:
                    _j = _j.concat([_m.sent()]);
                    return [4 /*yield*/, it.next()];
                case 15:
                    _f.apply(_e, [_h.apply(_g, [_j.concat([_m.sent()])])]);
                    mixed = [];
                    _m.label = 16;
                case 16:
                    _m.trys.push([16, 21, 22, 27]);
                    _k = __asyncValues([Promise.resolve("a"), "b", later("c")]);
                    _m.label = 17;
                case 17: return [4 /*yield*/, _k.next()];
                case 18:
                    if (!(_l = _m.sent(), !_l.done)) return [3 /*break*/, 20];
                    v = _l.value;
                    mixed.push(v);
                    _m.label = 19;
                case 19: return [3 /*break*/, 17];
                case 20: return [3 /*break*/, 27];
                case 21:
                    e_3_1 = _m.sent();
                    e_3 = { error: e_3_1 };
                    return [3 /*break*/, 27];
                case 22:
                    _m.trys.push([22, , 25, 26]);
                    if (!(_l && !_l.done && (_b = _k.return))) return [3 /*break*/, 24];
                    return [4 /*yield*/, _b.call(_k)];
                case 23:
                    _m.sent();
                    _m.label = 24;
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
