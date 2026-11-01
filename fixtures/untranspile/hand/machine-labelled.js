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
function grid(n) {
    var i, j;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                i = 0;
                _a.label = 1;
            case 1:
                if (!(i < n)) return [3 /*break*/, 6];
                j = 0;
                _a.label = 2;
            case 2:
                if (!(j < n)) return [3 /*break*/, 5];
                if (j > i)
                    return [3 /*break*/, 5];
                if (i * j > 3)
                    return [3 /*break*/, 6];
                return [4 /*yield*/, i + ":" + j];
            case 3:
                _a.sent();
                _a.label = 4;
            case 4:
                j++;
                return [3 /*break*/, 2];
            case 5:
                i++;
                return [3 /*break*/, 1];
            case 6: return [4 /*yield*/, "done"];
            case 7:
                _a.sent();
                return [2 /*return*/];
        }
    });
}
console.log(drain(grid(4)).join(" "));
function scan(rows) {
    var found, _i, rows_1, row, k;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                found = null;
                _i = 0, rows_1 = rows;
                _a.label = 1;
            case 1:
                if (!(_i < rows_1.length)) return [3 /*break*/, 5];
                row = rows_1[_i];
                k = 0;
                _a.label = 2;
            case 2:
                if (!(k < row.length)) return [3 /*break*/, 4];
                if (row[k] === "x") {
                    found = row;
                    return [3 /*break*/, 5];
                }
                return [4 /*yield*/, row[k]];
            case 3:
                _a.sent();
                k++;
                return [3 /*break*/, 2];
            case 4:
                _i++;
                return [3 /*break*/, 1];
            case 5: return [2 /*return*/, found];
        }
    });
}
var s = scan(["ab", "cxd", "ef"]);
var seen = [];
var step = s.next();
while (!step.done) {
    seen.push(step.value);
    step = s.next();
}
console.log(seen.join(""), step.value);
