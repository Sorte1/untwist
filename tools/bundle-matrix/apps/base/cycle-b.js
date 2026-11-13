console.log("eval cycle-b");
var a = require("./cycle-a.js");
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function () {
  return "b later " + a.late;
};
