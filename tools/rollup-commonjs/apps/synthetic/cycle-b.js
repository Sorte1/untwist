const log = require("./log");
log("b start");
const a = require("./cycle-a");
exports.name = "b";
exports.sawA = String(a.name) + "/" + String(a.fromB);
log("b end");
