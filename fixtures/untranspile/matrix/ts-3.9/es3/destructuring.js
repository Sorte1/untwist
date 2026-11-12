"use strict";
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
var __rest = (this && this.__rest) || function (s, e) {
    var t = {};
    for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p) && e.indexOf(p) < 0)
        t[p] = s[p];
    if (s != null && typeof Object.getOwnPropertySymbols === "function")
        for (var i = 0, p = Object.getOwnPropertySymbols(s); i < p.length; i++) {
            if (e.indexOf(p[i]) < 0 && Object.prototype.propertyIsEnumerable.call(s, p[i]))
                t[p[i]] = s[p[i]];
        }
    return t;
};
var _a;
var _b = [1, 2, undefined, 4, 5], a = _b[0], _c = _b[2], b = _c === void 0 ? 10 : _c, rest = _b.slice(3);
var _d = { x: 1, q: 9, r: 8 }, x = _d.x, _e = _d.y, why = _e === void 0 ? 2 : _e, _f = _d.z, deep = (_f === void 0 ? { deep: "d" } : _f).deep, others = __rest(_d, ["x", "y", "z"]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point(_a, _b) {
    var _c = _a === void 0 ? {} : _a, _d = _c.x, x = _d === void 0 ? 0 : _d, _e = _c.y, y = _e === void 0 ? 0 : _e;
    var _f = _b === void 0 ? [] : _b, first = _f[0], second = _f[1];
    return x + y + (first || 0) + (second || 0);
}
console.log(point({ x: 1 }, [2, 3]), point());
var m = 1;
var n = 2;
_a = [n, m], m = _a[0], n = _a[1];
console.log(m, n);
var pairs = [[1, "one"], [2, "two"]];
for (var _i = 0, pairs_1 = pairs; _i < pairs_1.length; _i++) {
    var _g = pairs_1[_i], num = _g[0], word = _g[1];
    console.log(num, word);
}
var length = "hello".length;
var _h = "hi", c1 = _h[0], c2 = _h[1];
console.log(length, c1, c2);
var data = { list: [{ id: 7, tags: ["t1", "t2"] }] };
var _j = data.list[0], id = _j.id, _k = _j.tags, second = _k[1];
console.log(id, second);
var key = "dyn";
var _l = {}, _m = key, _o = _l[_m], dynamic = _o === void 0 ? "fallback" : _o;
console.log(dynamic);
function swap(_a) {
    var p = _a[0], q = _a[1];
    return [q, p];
}
console.log(swap([1, 2]).join(","));
var _p = (function () { return __generator(this, function (_a) {
    switch (_a.label) {
        case 0: return [4 /*yield*/, "a"];
        case 1:
            _a.sent();
            return [4 /*yield*/, "b"];
        case 2:
            _a.sent();
            return [4 /*yield*/, "c"];
        case 3:
            _a.sent();
            return [2 /*return*/];
    }
}); })(), g1 = _p[0], g2 = _p[1];
console.log(g1, g2);
