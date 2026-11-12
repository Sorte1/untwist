"use strict";

var _cfg$retries, _cfg$name, _cfg$timeout, _ref, _side, _ref2, _ref3;
function pick(a, b) {
  return a != null ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
var cfg = {
  retries: 0,
  name: ""
};
var retries = (_cfg$retries = cfg.retries) != null ? _cfg$retries : 3;
var name = (_cfg$name = cfg.name) != null ? _cfg$name : "anon";
var missing = (_cfg$timeout = cfg.timeout) != null ? _cfg$timeout : 1000;
console.log(retries, name, missing);
var calls = 0;
var side = function side() {
  calls++;
  return null;
};
console.log((_ref = (_side = side()) != null ? _side : side()) != null ? _ref : "end", calls);
console.log(((_ref2 = null) != null ? _ref2 : 0) || "or", (_ref3 = null) != null ? _ref3 : 0 || "inner");
