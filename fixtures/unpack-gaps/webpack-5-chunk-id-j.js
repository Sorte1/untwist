(() => {
  var __webpack_modules__ = {
    801: (module) => {
      module.exports = "eight oh one";
    },
    544: (module, exports, __webpack_require__) => {
      exports.value = __webpack_require__(801);
    }
  };
  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cached = __webpack_module_cache__[moduleId];
    if (cached !== undefined) return cached.exports;
    var module = __webpack_module_cache__[moduleId] = { exports: {} };
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  __webpack_require__.d = (exports, definition) => {
    for (var key in definition) Object.defineProperty(exports, key, { get: definition[key] });
  };
  __webpack_require__.r = (exports) => {
    Object.defineProperty(exports, "__esModule", { value: true });
  };
  __webpack_require__.j = 544;
})();
