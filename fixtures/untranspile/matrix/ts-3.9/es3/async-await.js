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
var log = [];
var later = function (v) { return new Promise(function (resolve) { return setTimeout(function () { return resolve(v); }, 0); }); };
function fetchTwice(x) {
    return __awaiter(this, void 0, void 0, function () {
        var a, b;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    log.push("start " + x);
                    return [4 /*yield*/, later(x)];
                case 1:
                    a = _a.sent();
                    return [4 /*yield*/, later(a + 1)];
                case 2:
                    b = _a.sent();
                    return [2 /*return*/, a + b];
            }
        });
    });
}
function guarded() {
    return __awaiter(this, void 0, void 0, function () {
        var e_1;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    _a.trys.push([0, 2, 3, 4]);
                    return [4 /*yield*/, later(0)];
                case 1:
                    _a.sent();
                    throw new TypeError("t");
                case 2:
                    e_1 = _a.sent();
                    log.push("caught " + e_1.name);
                    return [2 /*return*/, "r"];
                case 3:
                    log.push("finally");
                    return [7 /*endfinally*/];
                case 4: return [2 /*return*/];
            }
        });
    });
}
var service = {
    base: 100,
    total: function (n) {
        return __awaiter(this, void 0, void 0, function () {
            var add, sum, i, _a;
            var _this = this;
            return __generator(this, function (_b) {
                switch (_b.label) {
                    case 0:
                        add = function (k) { return __awaiter(_this, arguments, void 0, function () { return __generator(this, function (_a) {
                            return [2 /*return*/, this.base + k + arguments.length];
                        }); }); };
                        sum = 0;
                        i = 0;
                        _b.label = 1;
                    case 1:
                        if (!(i < n)) return [3 /*break*/, 4];
                        _a = sum;
                        return [4 /*yield*/, add(i)];
                    case 2:
                        sum = _a + _b.sent();
                        _b.label = 3;
                    case 3:
                        i++;
                        return [3 /*break*/, 1];
                    case 4: return [2 /*return*/, sum];
                }
            });
        });
    }
};
var Loader = /** @class */ (function () {
    function Loader() {
        this.items = [];
    }
    Loader.prototype.load = function (v) {
        return __awaiter(this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, null];
                    case 1:
                        _a.sent();
                        this.items.push(v);
                        return [2 /*return*/, this.items.length];
                }
            });
        });
    };
    return Loader;
}());
function main() {
    return __awaiter(this, void 0, void 0, function () {
        var p, _a, _b, _c, l, _d, _e, _f, arrow, _g, _h, _j;
        var _this = this;
        return __generator(this, function (_k) {
            switch (_k.label) {
                case 0:
                    p = fetchTwice(1);
                    log.push("after call");
                    _b = (_a = console).log;
                    return [4 /*yield*/, p];
                case 1:
                    _c = [_k.sent()];
                    return [4 /*yield*/, guarded()];
                case 2:
                    _c = _c.concat([_k.sent()]);
                    return [4 /*yield*/, service.total(3)];
                case 3:
                    _b.apply(_a, _c.concat([_k.sent()]));
                    l = new Loader();
                    _e = (_d = console).log;
                    return [4 /*yield*/, l.load("a")];
                case 4:
                    _f = [_k.sent()];
                    return [4 /*yield*/, l.load("b")];
                case 5:
                    _e.apply(_d, _f.concat([_k.sent()]));
                    arrow = function (n) { return __awaiter(_this, void 0, void 0, function () { return __generator(this, function (_a) {
                        switch (_a.label) {
                            case 0: return [4 /*yield*/, later(n)];
                            case 1: return [2 /*return*/, (_a.sent()) * 2];
                        }
                    }); }); };
                    _h = (_g = console).log;
                    return [4 /*yield*/, arrow(4)];
                case 6:
                    _j = [_k.sent()];
                    return [4 /*yield*/, Promise.all([arrow(1), fetchTwice(2)])];
                case 7:
                    _h.apply(_g, _j.concat([(_k.sent()).join(",")]));
                    console.log(log.join("; "));
                    return [2 /*return*/];
            }
        });
    });
}
main()["catch"](function (e) { return console.log("fail", e); });
