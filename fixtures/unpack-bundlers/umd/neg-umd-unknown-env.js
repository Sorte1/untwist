(function (root, factory) {
  if (typeof window === "undefined") factory(exports);
  else factory((root.lib = {}));
})(this, function (exports) {
  console.log("eval umd env");
  exports.value = 9;
});
