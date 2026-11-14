(function (module) {
  module.exports = "id " + typeof module.id;
})(typeof module === "undefined" || module);
