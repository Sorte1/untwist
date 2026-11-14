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
    var lib = __webpack_require__(1);
    var os = require("os");
    var a = 1, b = 2, c = 3, d = 4; console.log(lib.go(), typeof os.EOL, a < b > (c, d));
  },
  function (module, exports) {
    exports.go = function () {
      function f(b) { console.log(typeof b); return b; }
      const r = f(2);
      f("x");
      return typeof r;
    };
  },
]);
