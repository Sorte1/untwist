var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// app/x.cjs
var require_x = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	import("./lazy.js").then(m => console.log("got", m.default), e => console.log("threw", e.code));
}));
console.log("eval index"); require_x();
export {};
