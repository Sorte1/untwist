(function (module) {
  module.exports = {
    require: typeof module.require,
    paths: module.paths.length,
    polyfilled: module.webpackPolyfill,
    deprecate: typeof module.deprecate,
    loadedLater: function () {
      return module.loaded;
    }
  };
})(typeof module === "undefined" || module);
