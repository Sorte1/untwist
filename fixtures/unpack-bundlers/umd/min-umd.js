(function (root, factory) {
  if (typeof define === "function" && define.amd) define(["exports"], factory);
  else if (typeof exports === "object" && typeof module !== "undefined") factory(exports);
  else factory((root.lib = {}));
})(this, function (exports) {
  "use strict";
  console.log("eval umd");
  exports.value = 5;
});
