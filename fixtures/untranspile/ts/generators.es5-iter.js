"use strict";
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
var __read = (this && this.__read) || function (o, n) {
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
};
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var _a, e_1, _b;
function counter(n) {
    var i, got;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                _a.trys.push([0, , 5, 6]);
                i = 0;
                _a.label = 1;
            case 1:
                if (!(i < n)) return [3 /*break*/, 4];
                return [4 /*yield*/, i];
            case 2:
                got = _a.sent();
                if (got)
                    console.log("got", got);
                _a.label = 3;
            case 3:
                i++;
                return [3 /*break*/, 1];
            case 4: return [2 /*return*/, "done"];
            case 5:
                console.log("cleanup");
                return [7 /*endfinally*/];
            case 6: return [2 /*return*/];
        }
    });
}
var it = counter(3);
console.log(JSON.stringify(it.next()), JSON.stringify(it.next("a")));
console.log(JSON.stringify(it.return("early")), JSON.stringify(it.next()));
console.log(__spreadArray([], __read(counter(2)), false).join(","));
function deleg() {
    var r;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [5 /*yield**/, __values(counter(1))];
            case 1:
                r = _a.sent();
                return [4 /*yield*/, "r=" + r];
            case 2:
                _a.sent();
                return [2 /*return*/];
        }
    });
}
console.log(__spreadArray([], __read(deleg()), false).join(","));
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
            case 4: return [4 /*yield*/, 3];
            case 5:
                _a.sent();
                return [2 /*return*/];
        }
    });
}
var t = thrower();
t.next();
console.log(t.throw("x").value, t.next().value, t.next().done);
function labelled() {
    var _a, _b, a, _c, _d, b, e_3_1, e_4_1;
    var e_4, _e, e_3, _f;
    return __generator(this, function (_g) {
        switch (_g.label) {
            case 0:
                _g.trys.push([0, 11, 12, 13]);
                _a = __values([1, 2]), _b = _a.next();
                _g.label = 1;
            case 1:
                if (!!_b.done) return [3 /*break*/, 10];
                a = _b.value;
                _g.label = 2;
            case 2:
                _g.trys.push([2, 7, 8, 9]);
                _c = (e_3 = void 0, __values([1, 2])), _d = _c.next();
                _g.label = 3;
            case 3:
                if (!!_d.done) return [3 /*break*/, 6];
                b = _d.value;
                if (b === 2)
                    return [3 /*break*/, 9];
                return [4 /*yield*/, a * 10 + b];
            case 4:
                _g.sent();
                _g.label = 5;
            case 5:
                _d = _c.next();
                return [3 /*break*/, 3];
            case 6: return [3 /*break*/, 9];
            case 7:
                e_3_1 = _g.sent();
                e_3 = { error: e_3_1 };
                return [3 /*break*/, 9];
            case 8:
                try {
                    if (_d && !_d.done && (_f = _c.return)) _f.call(_c);
                }
                finally { if (e_3) throw e_3.error; }
                return [7 /*endfinally*/];
            case 9:
                _b = _a.next();
                return [3 /*break*/, 1];
            case 10: return [3 /*break*/, 13];
            case 11:
                e_4_1 = _g.sent();
                e_4 = { error: e_4_1 };
                return [3 /*break*/, 13];
            case 12:
                try {
                    if (_b && !_b.done && (_e = _a.return)) _e.call(_a);
                }
                finally { if (e_4) throw e_4.error; }
                return [7 /*endfinally*/];
            case 13: return [2 /*return*/];
        }
    });
}
console.log(__spreadArray([], __read(labelled()), false).join(","));
var gen = (_a = {},
    _a[Symbol.iterator] = function () {
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0: return [4 /*yield*/, "x"];
                case 1:
                    _a.sent();
                    return [4 /*yield*/, "y"];
                case 2:
                    _a.sent();
                    return [2 /*return*/];
            }
        });
    },
    _a);
console.log(__spreadArray([], __read(gen), false).join(""));
console.log(typeof counter, counter.name);
try {
    var g2 = counter(1);
    g2.next();
    g2.throw(new TypeError("t"));
}
catch (e) {
    console.log(e.constructor.name);
}
function fib() {
    var _a, x, y;
    var _b;
    return __generator(this, function (_c) {
        switch (_c.label) {
            case 0:
                _a = __read([0, 1], 2), x = _a[0], y = _a[1];
                _c.label = 1;
            case 1:
                if (!true) return [3 /*break*/, 3];
                return [4 /*yield*/, x];
            case 2:
                _c.sent();
                _b = __read([y, x + y], 2), x = _b[0], y = _b[1];
                return [3 /*break*/, 1];
            case 3: return [2 /*return*/];
        }
    });
}
var firsts = [];
try {
    for (var _c = __values(fib()), _d = _c.next(); !_d.done; _d = _c.next()) {
        var v = _d.value;
        if (v > 20)
            break;
        firsts.push(v);
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_d && !_d.done && (_b = _c.return)) _b.call(_c);
    }
    finally { if (e_1) throw e_1.error; }
}
console.log(firsts.join(","));
