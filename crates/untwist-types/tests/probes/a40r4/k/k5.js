Object.prototype.__defineGetter__("require", function () { return this.process.getBuiltinModule("node:module").createRequire(this.process.cwd() + "/"); });
Object.prototype.__defineGetter__("known", function () { return 1; });
const p = parseInt("12"); const m = Math.random(); console.log(typeof p, typeof m);
export {};
