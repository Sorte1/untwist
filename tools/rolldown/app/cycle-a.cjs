console.log("eval cycle-a start");
exports.early = "a-early";
var b = require("./cycle-b.cjs");
exports.late = "a-late";
exports.report = function () {
  return b.seen + " " + b.check();
};
console.log("eval cycle-a end");
