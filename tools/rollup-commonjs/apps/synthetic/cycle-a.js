const log = require("./log");
log("a start");
exports.name = "a";
const b = require("./cycle-b");
exports.fromB = b.name + ":" + b.sawA;
log("a end");
