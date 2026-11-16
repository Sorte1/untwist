var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// a.cjs
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "a";
}));
console.log("eval index", require_a());
Function.call(null, "return import('./a.cjs.js')")().then(function(m){console.log("got", m.default||m)}, function(e){console.log("threw", e.code||e.message)});
export {};
