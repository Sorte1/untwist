(function (modules) {
  var cache = {};
  function __webpack_require__(id) {
    if (cache[id]) return cache[id].exports;
    var module = (cache[id] = { i: id, l: false, exports: {} });
    modules[id].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  return __webpack_require__(0);
})([
  function (module, exports, __webpack_require__) {
    __webpack_require__(1);
    var lib = __webpack_require__(2);
    console.log(lib.go());
  },
  function (module, exports) {
    Math.random = function () { return "patched"; };
    String.prototype.trim = function () { return 7; };
  },
  function (module, exports) {
    exports.go = function () {
      const r = Math.random();
      const t = String(r).trim();
      return typeof r + " " + typeof t;
    };
  },
]);
