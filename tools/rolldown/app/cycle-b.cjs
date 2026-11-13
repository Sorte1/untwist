console.log("eval cycle-b");
var a = require("./cycle-a.cjs");
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function () {
  return "b later " + a.late;
};
