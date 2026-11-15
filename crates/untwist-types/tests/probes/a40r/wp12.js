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
    var os = require("os"); var again = Promise.resolve().then(__webpack_require__.bind(null, 1)); again.then(function (m) { console.log(typeof m.go); });
    console.log(lib.go(), typeof os.EOL);
  },
  function (module, exports) {
    exports.go = function () {
      return "plain";
    };
  },
]);
