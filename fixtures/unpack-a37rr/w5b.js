(() => {
  var __webpack_modules__ = {
    1: (module) => { module.exports = function (module) { if (!module.webpackPolyfill) { module.deprecate = function () {}; module.paths = []; if (!module.children) module.children = []; Object.defineProperty(module, "loaded", { enumerable: true, get: function () { return module.l; } }); Object.defineProperty(module, "id", { enumerable: true, get: function () { return module.i; } }); module.webpackPolyfill = 1; } return module; }; },
    2: (module, exports, __webpack_require__) => { exports.two = 2; }
  };
  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cachedModule = __webpack_module_cache__[moduleId];
    if (cachedModule !== undefined) { return cachedModule.exports; }
    var module = __webpack_module_cache__[moduleId] = { exports: {} };
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  var __webpack_exports__ = {};
  (() => {
    var two = __webpack_require__(2); console.log(two.two, typeof module.filename, typeof module.i, module.loaded);
  })();
})();
