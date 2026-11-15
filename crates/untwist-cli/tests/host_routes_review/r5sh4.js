import { createRequire } from "node:module";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
var __require = /* #__PURE__ */ (() => createRequire(import.meta.url))();
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
console.log("eval index", require_a());
try { console.log("got", __require("./a.cjs.js")); } catch (e) { console.log("threw", e.code || e.message); }
try { console.log("got", __require.resolve("./a.cjs.js").split("/").pop()); } catch (e) { console.log("threw", e.code || e.message); }
export {};
