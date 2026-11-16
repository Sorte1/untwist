var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// c.cjs
var require_c = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "c";
}));
console.log("eval chunk", require_c());
export default "chunk";
