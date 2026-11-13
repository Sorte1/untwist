(function() {

// \0rolldown/runtime.js
	var __defProp = Object.defineProperty;
	var __esmMin = (fn, res, err) => () => {
		if (err) throw err[0];
		try {
			return fn && (res = fn(fn = 0)), res;
		} catch (e) {
			throw err = [e], e;
		}
	};
	var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
	var __exportAll = (all, flags) => {
		let target = Object.setPrototypeOf({}, null);
		for (var name in all) {
			__defProp(target, name, {
				get: all[name],
				enumerable: true
			});
		}
		if (!(flags & 1)) {
			__defProp(target, Symbol.toStringTag, { value: "Module" });
		}
		return flags & 2 ? target : Object.preventExtensions(target);
	};

// .src-mini-esm/cjs.js
	var require_cjs = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cjs");
		exports.twice = function(n) {
			return n * 2;
		};
	}));

// .src-mini-esm/lazy.js
	var lazy_exports = /* @__PURE__ */ __exportAll({ value: () => 42 });
	var value;
	var init_lazy = __esmMin((() => {
		console.log("eval lazy");
		value = 42;
	}));

// .src-mini-esm/index.js
	var import_cjs = require_cjs();
	console.log("eval index " + (0, import_cjs.twice)(21));
	Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => console.log("lazy " + lazy.value));

})();