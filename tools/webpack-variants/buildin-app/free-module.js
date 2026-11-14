(function (root) {
  var freeModule = typeof module == "object" && module && !module.nodeType && module;
  var api = { version: "1" };
  if (typeof define == "function" && typeof define.amd == "object" && define.amd) {
    define("api", function () {
      return api;
    });
  } else if (freeModule) {
    freeModule.exports = api;
  }
})(this);
