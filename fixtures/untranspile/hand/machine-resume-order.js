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
var log = [];
var g;
function operandWritesGlobal() {
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                g = 1;
                return [4 /*yield*/, (g = 2)];
            case 1:
                return [2 /*return*/, g + _a.sent()];
        }
    });
}
var it = operandWritesGlobal();
it.next();
log.push(it.next(5).value);
var h = 1;
function globalChangesWhileSuspended() {
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, 0];
            case 1:
                return [2 /*return*/, h + _a.sent()];
        }
    });
}
it = globalChangesWhileSuspended();
it.next();
h = 10;
log.push(it.next(5).value);
function operandWritesLocal() {
    var x;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                x = 1;
                return [4 /*yield*/, (x = 2)];
            case 1:
                return [2 /*return*/, x + _a.sent()];
        }
    });
}
it = operandWritesLocal();
it.next();
log.push(it.next(5).value);
var hook;
function capturedLocal() {
    var n;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                n = 1;
                hook = function () { n = 10; };
                return [4 /*yield*/, 0];
            case 1:
                return [2 /*return*/, n + _a.sent()];
        }
    });
}
it = capturedLocal();
it.next();
hook();
log.push(it.next(5).value);
var shared = { v: 1 };
function memberRead() {
    var o;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                o = shared;
                return [4 /*yield*/, 0];
            case 1:
                return [2 /*return*/, o.v + _a.sent()];
        }
    });
}
it = memberRead();
it.next();
shared.v = 10;
log.push(it.next(5).value);
var total = 1;
function compoundGlobal() {
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0: return [4 /*yield*/, 0];
            case 1:
                total += _a.sent();
                return [2 /*return*/, total];
        }
    });
}
it = compoundGlobal();
it.next();
total = 10;
log.push(it.next(5).value);
function steadyLocal(a) {
    var b;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                b = 3;
                return [4 /*yield*/, 0];
            case 1:
                b += a + _a.sent();
                return [2 /*return*/, b];
        }
    });
}
it = steadyLocal(2);
it.next();
log.push(it.next(5).value);
console.log(log.join(" "));
