"use strict";

var _cfg$retries, _cfg$name, _cfg$timeout, _ref, _side, _ref2, _ref3;
function pick(a, b) {
  return a !== null && a !== void 0 ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
const cfg = {
  retries: 0,
  name: ""
};
const retries = (_cfg$retries = cfg.retries) !== null && _cfg$retries !== void 0 ? _cfg$retries : 3;
const name = (_cfg$name = cfg.name) !== null && _cfg$name !== void 0 ? _cfg$name : "anon";
const missing = (_cfg$timeout = cfg.timeout) !== null && _cfg$timeout !== void 0 ? _cfg$timeout : 1000;
console.log(retries, name, missing);
let calls = 0;
const side = () => {
  calls++;
  return null;
};
console.log((_ref = (_side = side()) !== null && _side !== void 0 ? _side : side()) !== null && _ref !== void 0 ? _ref : "end", calls);
console.log(((_ref2 = null) !== null && _ref2 !== void 0 ? _ref2 : 0) || "or", (_ref3 = null) !== null && _ref3 !== void 0 ? _ref3 : 0 || "inner");
