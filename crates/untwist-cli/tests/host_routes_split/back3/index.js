import { tag } from "./shared.chunk.js";
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	console.log("eval a");
	module.exports = { n: 1 };
}));
var a = require_a();
console.log("eval index", a.n, tag);
let count = 0;
function bump() { return ++count; }
setTimeout(() => console.log("count", count), 10);
export { bump };
