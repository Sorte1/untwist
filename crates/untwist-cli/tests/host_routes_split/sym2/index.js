var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
console.log("eval index", require_a());
import("./lazy.chunk.js").then(() => import("./alias.js")).then(m => console.log("got", m.default), e => console.log("threw", e.code));
export {};
