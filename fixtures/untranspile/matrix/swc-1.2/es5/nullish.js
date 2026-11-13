"use strict";
function pick(a, b) {
    return a !== null && a !== void 0 ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
var cfg = {
    retries: 0,
    name: ""
};
var _retries;
var retries = (_retries = cfg.retries) !== null && _retries !== void 0 ? _retries : 3;
var _name;
var name = (_name = cfg.name) !== null && _name !== void 0 ? _name : "anon";
var _timeout;
var missing = (_timeout = cfg.timeout) !== null && _timeout !== void 0 ? _timeout : 1000;
console.log(retries, name, missing);
var calls = 0;
var side = function() {
    calls++;
    return null;
};
var ref, ref1;
console.log((ref1 = (ref = side()) !== null && ref !== void 0 ? ref : side()) !== null && ref1 !== void 0 ? ref1 : "end", calls);
var ref2, ref3;
console.log(((ref2 = null) !== null && ref2 !== void 0 ? ref2 : 0) || "or", (ref3 = null) !== null && ref3 !== void 0 ? ref3 : 0 || "inner");
