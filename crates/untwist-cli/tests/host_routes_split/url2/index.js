var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// lazy.js
var require_lazy = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	console.log("eval bundled lazy");
	module.exports = "bundled lazy";
}));
console.log("eval index", require_lazy());
import("./lazy.js?x").then(m => console.log("got", m.default), e => console.log("threw", e.code));
export {};
