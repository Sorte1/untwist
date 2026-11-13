var helper = require("./helper.cjs");
console.log("legacy evaluated");
exports.twice = function (n) {
  return helper.scale(n, 2);
};
exports.name = "legacy";
