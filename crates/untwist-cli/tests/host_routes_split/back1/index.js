var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
var require_a = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	console.log("eval a");
	module.exports = { n: 1 };
}));
var a = require_a();
console.log("eval index", a.n);
let count = 0;
function bump() { return ++count; }
import("./lazy.chunk.js").then(m => { console.log("lazy", m.default, count); });
export { bump };
