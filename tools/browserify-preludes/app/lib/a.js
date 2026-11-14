exports.name = "a";
exports.count = 1;
var b = require("./b");
console.log("a sees b", typeof b.describe);
