var matrix = (function () {
	'use strict';

	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
	}

	var _srcMiniCjs = {};

	var cjs = {};

	var hasRequiredCjs;

	function requireCjs () {
		if (hasRequiredCjs) return cjs;
		hasRequiredCjs = 1;
		console.log("eval cjs");
		cjs.twice = function (n) {
		  return n * 2;
		};
		return cjs;
	}

	var lazy = {};

	var hasRequiredLazy;

	function requireLazy () {
		if (hasRequiredLazy) return lazy;
		hasRequiredLazy = 1;

		Object.defineProperty(lazy, "__esModule", {
		  value: true
		});
		lazy.value = void 0;
		console.log("eval lazy");
		lazy.value = 42;
		return lazy;
	}

	var hasRequired_srcMiniCjs;

	function require_srcMiniCjs () {
		if (hasRequired_srcMiniCjs) return _srcMiniCjs;
		hasRequired_srcMiniCjs = 1;

		var _cjs = requireCjs();
		var cache;
		function _interopRequireWildcard(e, t) { if (e && e.__esModule) return e; var r, c, a = Object.defineProperty, n = { __proto__: null, default: e }; if (Object(e) !== e) return n; if (cache || "function" != typeof WeakMap || (cache = new WeakMap()), cache) { if (cache.has(e)) return cache.get(e); cache.set(e, n); } for (c in e) "default" !== c && {}.hasOwnProperty.call(e, c) && ((r = a && Object.getOwnPropertyDescriptor(e, c)) && (r.get || r.set) ? a(n, c, r) : n[c] = e[c]); return n; }
		console.log("eval index " + (0, _cjs.twice)(21));
		Promise.resolve().then(() => _interopRequireWildcard(requireLazy())).then(lazy => console.log("lazy " + lazy.value));
		return _srcMiniCjs;
	}

	var _srcMiniCjsExports = require_srcMiniCjs();
	var index = /*@__PURE__*/getDefaultExportFromCjs(_srcMiniCjsExports);

	return index;

})();
