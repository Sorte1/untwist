"use strict";
var label = "strict";
function mode() {
  return typeof this === "undefined" ? "strict" : "sloppy";
}
exports.mode = mode;
exports.label = label;
