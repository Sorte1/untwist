var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
console.log("eval index", require_a(), import.meta.url.split("/").pop(), import.meta.filename.split("/").pop());
try { console.log(import.meta.resolve("./a.cjs.js").split("/").pop()); } catch (e) { console.log("threw", e.code); }
export {};
