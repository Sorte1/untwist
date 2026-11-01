(function (root, factory) {
  if (typeof define === "function" && define.amd) {
    define(["path"], factory);
  } else if (typeof module === "object" && module.exports) {
    module.exports = factory(require("path"));
  } else {
    root.lib = factory(root.path);
  }
})(typeof self !== "undefined" ? self : this, function (path) {
  console.log("eval umd deps", path.posix.join("a", "b"));
  return { joined: true };
});
