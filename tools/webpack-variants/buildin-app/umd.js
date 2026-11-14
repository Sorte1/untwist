(function (module, exports) {
  "use strict";
  function Big(n) {
    this.n = n;
  }
  Big.prototype.toString = function () {
    return "Big(" + this.n + ")";
  };
  if (typeof module === "object") {
    module.exports = Big;
  } else {
    exports.Big = Big;
  }
})(typeof module === "undefined" || module, this);
