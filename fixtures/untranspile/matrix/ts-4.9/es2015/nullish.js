"use strict";
var _a, _b, _c, _d, _e;
function pick(a, b) {
    return a !== null && a !== void 0 ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
const cfg = { retries: 0, name: "" };
const retries = (_a = cfg.retries) !== null && _a !== void 0 ? _a : 3;
const name = (_b = cfg.name) !== null && _b !== void 0 ? _b : "anon";
const missing = (_c = cfg.timeout) !== null && _c !== void 0 ? _c : 1000;
console.log(retries, name, missing);
let calls = 0;
const side = () => {
    calls++;
    return null;
};
console.log((_e = (_d = side()) !== null && _d !== void 0 ? _d : side()) !== null && _e !== void 0 ? _e : "end", calls);
console.log((null !== null && null !== void 0 ? null : 0) || "or", null !== null && null !== void 0 ? null : (0 || "inner"));
