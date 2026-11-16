import { createRequire } from "node:module";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
const r = createRequire(import.meta.url);
console.log("eval index", require_a());
try { console.log("got", r("./a.cjs.js")); } catch (e) { console.log("threw", e.code || e.message); }
export {};
