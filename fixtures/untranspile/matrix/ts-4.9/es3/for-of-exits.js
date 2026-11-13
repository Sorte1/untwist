"use strict";
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
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
var log = [];
function counted(name, items) {
    var _a;
    return _a = {},
        _a[Symbol.iterator] = function () {
            var at = 0;
            log.push(name + ":open");
            return {
                next: function () {
                    log.push(name + ":next");
                    return at < items.length ? { value: items[at++], done: false } : { value: undefined, done: true };
                },
                "return": function () {
                    log.push(name + ":return");
                    return { done: true };
                }
            };
        },
        _a;
}
for (var _i = 0, _a = counted("all", [1, 2]); _i < _a.length; _i++) {
    var v = _a[_i];
    log.push("all=" + v);
}
for (var _b = 0, _c = counted("brk", [1, 2, 3]); _b < _c.length; _b++) {
    var v = _c[_b];
    if (v === 2)
        break;
    log.push("brk=" + v);
}
function early() {
    for (var _i = 0, _a = counted("ret", [5, 6, 7]); _i < _a.length; _i++) {
        var v = _a[_i];
        if (v === 6)
            return v;
    }
    return -1;
}
log.push("early=" + early());
try {
    for (var _d = 0, _e = counted("thr", [1, 2]); _d < _e.length; _d++) {
        var v = _e[_d];
        if (v === 2)
            throw new Error("boom " + v);
    }
}
catch (err) {
    log.push("caught " + err.message);
}
var skipped = 0;
for (var _f = 0, _g = counted("cont", [1, 2, 3]); _f < _g.length; _f++) {
    var v = _g[_f];
    if (v === 2)
        continue;
    skipped += v;
}
log.push("cont=" + skipped);
outer: for (var _h = 0, _j = counted("outer", [1, 2]); _h < _j.length; _h++) {
    var a = _j[_h];
    for (var _k = 0, _l = counted("inner", [10, 20]); _k < _l.length; _k++) {
        var b = _l[_k];
        if (b === 20)
            continue outer;
        log.push(a + b);
    }
}
var last;
for (var _m = 0, _o = counted("assign", [7, 8]); _m < _o.length; _m++) {
    last = _o[_m];
    log.push("seen " + last);
}
log.push("last=" + last);
var pairs = new Map([["x", 1], ["y", 2]]);
for (var _p = 0, pairs_1 = pairs; _p < pairs_1.length; _p++) {
    var _q = pairs_1[_p], key = _q[0], value = _q[1];
    log.push(key + value);
}
var fns = [];
var _loop_1 = function (v) {
    fns.push(function () { return v; });
};
for (var _r = 0, _s = [1, 2, 3]; _r < _s.length; _r++) {
    var v = _s[_r];
    _loop_1(v);
}
log.push(fns.map(function (f) { return f(); }).join(""));
function gen() {
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                _a.trys.push([0, , 3, 4]);
                return [4 /*yield*/, 1];
            case 1:
                _a.sent();
                return [4 /*yield*/, 2];
            case 2:
                _a.sent();
                return [3 /*break*/, 4];
            case 3:
                log.push("gen:finally");
                return [7 /*endfinally*/];
            case 4: return [2 /*return*/];
        }
    });
}
for (var _t = 0, _u = gen(); _t < _u.length; _t++) {
    var v = _u[_t];
    if (v === 1)
        break;
}
try {
    for (var _v = 0, _w = counted("bad", [1]); _v < _w.length; _v++) {
        var v = _w[_v];
        log.push(v.missing.field);
    }
}
catch (err) {
    log.push(err.constructor.name);
}
var spreadSet = __spreadArray([], new Set([4, 4, 5]), true);
var firstKey = new Map([["k", 1]]).keys()[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
