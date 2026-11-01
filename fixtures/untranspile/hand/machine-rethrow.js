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
function drain(it) {
    var out = [];
    var step = it.next();
    while (!step.done) {
        out.push(step.value);
        step = it.next();
    }
    return out;
}
function g(mode) {
    var e_1, e_2;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                _a.trys.push([0, 7, , 9]);
                _a.label = 1;
            case 1:
                _a.trys.push([1, 4, 5, 6]);
                return [4 /*yield*/, "start"];
            case 2:
                _a.sent();
                if (mode)
                    throw new RangeError("inner");
                return [4 /*yield*/, "middle"];
            case 3:
                _a.sent();
                return [3 /*break*/, 6];
            case 4:
                e_1 = _a.sent();
                console.log("inner catch", e_1.constructor.name);
                throw new TypeError("again");
            case 5:
                console.log("inner finally");
                return [7 /*endfinally*/];
            case 6: return [3 /*break*/, 9];
            case 7:
                e_2 = _a.sent();
                return [4 /*yield*/, "outer " + e_2.constructor.name];
            case 8:
                _a.sent();
                return [3 /*break*/, 9];
            case 9: return [2 /*return*/, "end"];
        }
    });
}
console.log(drain(g(true)).join(","));
console.log(drain(g(false)).join(","));
var it = g(false);
it.next();
console.log(JSON.stringify(it.throw(new SyntaxError("x"))), JSON.stringify(it.next()));
function escape() {
    var e_3;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                _a.trys.push([0, 2, , 3]);
                return [4 /*yield*/, 1];
            case 1:
                _a.sent();
                return [3 /*break*/, 3];
            case 2:
                e_3 = _a.sent();
                throw e_3;
            case 3: return [2 /*return*/];
        }
    });
}
try {
    var e = escape();
    e.next();
    e.throw(new EvalError("out"));
}
catch (err) {
    console.log("escaped", err.constructor.name);
}
