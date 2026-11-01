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
};
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
    return __asyncGenerator(this, arguments, function ticks_1() {
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
                    return [4 /*yield*/, __await(i)];
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
function main() {
    return __awaiter(this, void 0, void 0, function () {
        var out, _a, _b, _c, v, e_1_1, it, _d, _e, _f, _g, _h, _j, _k, _l, _m, fromSync, _o, _p, _q, v, e_2_1;
        var _r, e_1, _s, _t, _u, e_2, _v, _w;
        return __generator(this, function (_x) {
            switch (_x.label) {
                case 0:
                    out = [];
                    _x.label = 1;
                case 1:
                    _x.trys.push([1, 6, 7, 12]);
                    _a = true, _b = __asyncValues(ticks(3));
                    _x.label = 2;
                case 2: return [4 /*yield*/, _b.next()];
                case 3:
                    if (!(_c = _x.sent(), _r = _c.done, !_r)) return [3 /*break*/, 5];
                    _t = _c.value;
                    _a = false;
                    v = _t;
                    out.push(v);
                    _x.label = 4;
                case 4:
                    _a = true;
                    return [3 /*break*/, 2];
                case 5: return [3 /*break*/, 12];
                case 6:
                    e_1_1 = _x.sent();
                    e_1 = { error: e_1_1 };
                    return [3 /*break*/, 12];
                case 7:
                    _x.trys.push([7, , 10, 11]);
                    if (!(!_a && !_r && (_s = _b.return))) return [3 /*break*/, 9];
                    return [4 /*yield*/, _s.call(_b)];
                case 8:
                    _x.sent();
                    _x.label = 9;
                case 9: return [3 /*break*/, 11];
                case 10:
                    if (e_1) throw e_1.error;
                    return [7 /*endfinally*/];
                case 11: return [7 /*endfinally*/];
                case 12:
                    console.log(out.join(","));
                    it = ticks(2);
                    _e = (_d = console).log;
                    _g = (_f = JSON).stringify;
                    return [4 /*yield*/, it.next()];
                case 13:
                    _h = [_g.apply(_f, [_x.sent()])];
                    _k = (_j = JSON).stringify;
                    return [4 /*yield*/, it.return(9)];
                case 14:
                    _h = _h.concat([_k.apply(_j, [_x.sent()])]);
                    _m = (_l = JSON).stringify;
                    return [4 /*yield*/, it.next()];
                case 15:
                    _e.apply(_d, _h.concat([_m.apply(_l, [_x.sent()])]));
                    fromSync = [];
                    _x.label = 16;
                case 16:
                    _x.trys.push([16, 21, 22, 27]);
                    _o = true, _p = __asyncValues([Promise.resolve("a"), "b"]);
                    _x.label = 17;
                case 17: return [4 /*yield*/, _p.next()];
                case 18:
                    if (!(_q = _x.sent(), _u = _q.done, !_u)) return [3 /*break*/, 20];
                    _w = _q.value;
                    _o = false;
                    v = _w;
                    fromSync.push(v);
                    _x.label = 19;
                case 19:
                    _o = true;
                    return [3 /*break*/, 17];
                case 20: return [3 /*break*/, 27];
                case 21:
                    e_2_1 = _x.sent();
                    e_2 = { error: e_2_1 };
                    return [3 /*break*/, 27];
                case 22:
                    _x.trys.push([22, , 25, 26]);
                    if (!(!_o && !_u && (_v = _p.return))) return [3 /*break*/, 24];
                    return [4 /*yield*/, _v.call(_p)];
                case 23:
                    _x.sent();
                    _x.label = 24;
                case 24: return [3 /*break*/, 26];
                case 25:
                    if (e_2) throw e_2.error;
                    return [7 /*endfinally*/];
                case 26: return [7 /*endfinally*/];
                case 27:
                    console.log(fromSync.join(""));
                    return [2 /*return*/];
            }
        });
    });
}
main();
