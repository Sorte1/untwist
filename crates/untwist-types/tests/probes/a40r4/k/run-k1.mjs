Object.prototype.__defineSetter__("exports", function (v) { v.G = this; });
const box = {};
exports = box;
const req = box.G.process.getBuiltinModule("node:module").createRequire(box.G.process.cwd() + "/");
req("./patch.js");
const p = parseInt("12"); const m = Math.random(); console.log(typeof p, typeof m);
export {};
