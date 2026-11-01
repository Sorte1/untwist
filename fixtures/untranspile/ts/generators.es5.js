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
var _a;
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
console.log(__spreadArray([], counter(2), true).join(","));
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
console.log(__spreadArray([], deleg(), true).join(","));
function thrower() {
    var e_1;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                _a.trys.push([0, 2, , 4]);
                return [4 /*yield*/, 1];
            case 1:
                _a.sent();
                return [3 /*break*/, 4];
            case 2:
                e_1 = _a.sent();
                return [4 /*yield*/, "caught " + e_1];
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
    var _i, _a, a, _b, _c, b;
    return __generator(this, function (_d) {
        switch (_d.label) {
            case 0:
                _i = 0, _a = [1, 2];
                _d.label = 1;
            case 1:
                if (!(_i < _a.length)) return [3 /*break*/, 6];
                a = _a[_i];
                _b = 0, _c = [1, 2];
                _d.label = 2;
            case 2:
                if (!(_b < _c.length)) return [3 /*break*/, 5];
                b = _c[_b];
                if (b === 2)
                    return [3 /*break*/, 5];
                return [4 /*yield*/, a * 10 + b];
            case 3:
                _d.sent();
                _d.label = 4;
            case 4:
                _b++;
                return [3 /*break*/, 2];
            case 5:
                _i++;
                return [3 /*break*/, 1];
            case 6: return [2 /*return*/];
        }
    });
}
console.log(__spreadArray([], labelled(), true).join(","));
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
console.log(__spreadArray([], gen, true).join(""));
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
                _a = [0, 1], x = _a[0], y = _a[1];
                _c.label = 1;
            case 1:
                if (!true) return [3 /*break*/, 3];
                return [4 /*yield*/, x];
            case 2:
                _c.sent();
                _b = [y, x + y], x = _b[0], y = _b[1];
                return [3 /*break*/, 1];
            case 3: return [2 /*return*/];
        }
    });
}
var firsts = [];
for (var _i = 0, _b = fib(); _i < _b.length; _i++) {
    var v = _b[_i];
    if (v > 20)
        break;
    firsts.push(v);
}
console.log(firsts.join(","));
