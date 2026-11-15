(function (root, factory) {
  if (typeof define === "function" && define.amd) define(["exports", "module"], factory);
  else if (typeof exports === "object" && typeof module !== "undefined") factory(exports, module);
  else factory((root.lib = {}), {});
})(this, function (exports, module) {
  "use strict";
  var keep = module.exports; module.exports = function () { return typeof this.filename + " " + Object.keys(this).length; }; console.log(module.exports()); module.exports = keep;
  exports.value = 5;
});
