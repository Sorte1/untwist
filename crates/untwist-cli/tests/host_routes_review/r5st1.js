import foo from "foo";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
// node_modules/foo/index.js
var require_foo = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = "bundled foo";
}));
console.log("eval index", require_foo(), foo);
