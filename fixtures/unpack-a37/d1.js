(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  return __webpack_require__((__webpack_require__.s = 0));
})
([
  function (e, t, n) {
    var _0x1a2b = n(1);
    function _0x3c4d(_0x5e6f) { return eval("typeof world") + eval("typeof index") + eval("typeof result") + _0x5e6f; }
    console.log(_0x3c4d(_0x1a2b), eval("typeof _0x1a2b2"));
  },
  function (e) { e.exports = "world"; },
]);
