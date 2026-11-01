(function (factory, root) {
  if (typeof define === "function" && define.amd) define(["exports"], factory);
  else if (typeof exports === "object" && typeof module !== "undefined") factory(exports);
  else factory((root.lib = {}));
})(function (exports) {
  "use strict";
  console.log("eval umd");
  exports.value = 6;
}, this);
