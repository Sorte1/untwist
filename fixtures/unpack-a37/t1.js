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
    var seen;
    try { seen = typeof eval("require;").cache; } catch (error) { seen = error.name; }
    console.log(seen, n(1));
  },
  function (e) { e.exports = "world"; },
]);
