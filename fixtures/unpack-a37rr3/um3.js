"use strict";
(function (root, factory) {
  if (typeof define === "function" && define.amd) define(["exports", "module"], factory);
  else if (typeof exports === "object" && typeof module !== "undefined") factory(exports, module);
  else factory((root.lib = {}), {});
})(this, function (exports, module) {
  class Q { [console.log("k", typeof this)]() {} }
  exports.value = 5;
});

