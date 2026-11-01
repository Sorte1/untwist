(function (root, factory) {
  if (typeof exports === "object" && typeof module !== "undefined") factory(exports);
  else factory((root.first = {}));
})(this, function (exports) {
  console.log("eval first");
  exports.first = 1;
});
(function (root, factory) {
  if (typeof exports === "object" && typeof module !== "undefined") factory(exports);
  else factory((root.second = {}));
})(this, function (exports) {
  console.log("eval second", exports.first);
});
