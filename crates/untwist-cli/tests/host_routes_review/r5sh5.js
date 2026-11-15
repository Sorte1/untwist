import { createRequire } from "node:module";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
var __require = /* #__PURE__ */ (() => createRequire(import.meta.url))();
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
// b.cjs
var require_b = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	try { console.log("got", typeof module.constructor._load, typeof module.paths, typeof module.require, module.id); } catch (e) { console.log("threw", e.code || e.message); }
}));
console.log("eval index", require_a()); require_b();
export {};
