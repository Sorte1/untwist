import M from "module";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
console.log("eval index", require_a());
try { console.log("got", M.createRequire(import.meta.url)("./a.cjs.js")); } catch (e) { console.log("threw", e.code || e.message); }
export {};
