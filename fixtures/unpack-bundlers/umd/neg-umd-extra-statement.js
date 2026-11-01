(function (root, factory) {
  var seen = typeof root;
  if (typeof exports === "object" && typeof module !== "undefined") factory(exports, seen);
  else factory((root.lib = {}), seen);
})(this, function (exports) {
  console.log("eval umd extra");
  exports.value = 7;
});
