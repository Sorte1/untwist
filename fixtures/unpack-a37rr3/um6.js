"use strict";
(function (root, factory) {
  if (typeof define === "function" && define.amd) define(["exports", "module"], factory);
  else if (typeof exports === "object" && typeof module !== "undefined") factory(exports, module);
  else factory((root.lib = {}), {});
})(this, function (exports, module) {
  console.log("f");
  exports.value = 5;
});
console.log("after", module.exports.value, typeof lib);
