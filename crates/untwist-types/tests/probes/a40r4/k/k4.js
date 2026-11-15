Object.prototype.__defineGetter__("module", function () { return { exports: this }; });
const req = module.exports.process.getBuiltinModule("node:module").createRequire(module.exports.process.cwd() + "/");
req("./patch.js");
const p = parseInt("12"); const m = Math.random(); console.log(typeof p, typeof m);
export {};
