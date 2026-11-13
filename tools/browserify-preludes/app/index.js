var a = require("./lib/a");
var b = require("./lib/b.js");
console.log("entry", a.name, b.describe(), this === module.exports);
module.exports = { total: a.count + b.count };
console.log("entry exports", JSON.stringify(module.exports));
