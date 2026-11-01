(function (root, factory) {
  if (typeof exports === "object" && typeof module !== "undefined") factory(exports, require("./neg-umd-relative-require.js"));
  else factory((root.lib = {}), root.dep);
})(this, function (exports, self) {
  console.log("eval umd relative", typeof self);
  exports.value = 8;
});
