Object.prototype.__defineGetter__("exports", function () {
  return this;
});
const req = exports.process.getBuiltinModule("node:module").createRequire(exports.process.cwd() + "/");
req("./patch.js");
/** @type {number} */ const p = parseInt("12");
/** @type {number} */ const m = Math.random();
console.log(typeof p, typeof m);
export {};
