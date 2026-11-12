// \0rolldown/runtime.js
var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);

// .src-mini-esm/cjs.js
var require_cjs = /* @__PURE__ */ __commonJSMin(((exports) => {
	console.log("eval cjs");
	exports.twice = function(n) {
		return n * 2;
	};
}));

// .src-mini-esm/index.js
var import_cjs = require_cjs();
console.log("eval index " + (0, import_cjs.twice)(21));
import("./lazy.chunk.js").then((lazy) => console.log("lazy " + lazy.value));

export {  };