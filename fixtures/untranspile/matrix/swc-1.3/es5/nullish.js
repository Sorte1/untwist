"use strict";
function pick(a, b) {
    return a !== null && a !== void 0 ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
var cfg = {
    retries: 0,
    name: ""
};
var _cfg_retries;
var retries = (_cfg_retries = cfg.retries) !== null && _cfg_retries !== void 0 ? _cfg_retries : 3;
var _cfg_name;
var name = (_cfg_name = cfg.name) !== null && _cfg_name !== void 0 ? _cfg_name : "anon";
var _cfg_timeout;
var missing = (_cfg_timeout = cfg.timeout) !== null && _cfg_timeout !== void 0 ? _cfg_timeout : 1000;
console.log(retries, name, missing);
var calls = 0;
var side = function() {
    calls++;
    return null;
};
var _side, _ref;
console.log((_ref = (_side = side()) !== null && _side !== void 0 ? _side : side()) !== null && _ref !== void 0 ? _ref : "end", calls);
var _ref1, _ref2;
console.log(((_ref1 = null) !== null && _ref1 !== void 0 ? _ref1 : 0) || "or", (_ref2 = null) !== null && _ref2 !== void 0 ? _ref2 : 0 || "inner");
