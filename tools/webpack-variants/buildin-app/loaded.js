(function (module) {
  var atLoad = module.loaded;
  module.exports = {
    atLoad: atLoad,
    later: function () {
      return module.loaded;
    }
  };
})(typeof module === "undefined" || module);
