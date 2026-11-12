var app = (function () {
	'use strict';

	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
	}

	function getAugmentedNamespace(n) {
	  if (Object.prototype.hasOwnProperty.call(n, '__esModule')) return n;
	  var f = n.default;
		if (typeof f == "function") {
			var a = function a () {
				var isInstance = false;
	      try {
	        isInstance = this instanceof a;
	      } catch (e) {}
				if (isInstance) {
	        return Reflect.construct(f, arguments, this.constructor);
				}
				return f.apply(this, arguments);
			};
			a.prototype = f.prototype;
	  } else a = {};
	  Object.defineProperty(a, '__esModule', {value: true});
		Object.keys(n).forEach(function (k) {
			var d = Object.getOwnPropertyDescriptor(n, k);
			Object.defineProperty(a, k, d.get ? d : {
				enumerable: true,
				get: function () {
					return n[k];
				}
			});
		});
		return a;
	}

	var requireEsm$1 = {};

	var log$1;
	var hasRequiredLog;

	function requireLog () {
		if (hasRequiredLog) return log$1;
		hasRequiredLog = 1;
		var count = 0;
		log$1 = function log(line) {
		  count += 1;
		  console.log(count + " " + line);
		};
		return log$1;
	}

	var logExports = requireLog();
	var log = /*@__PURE__*/getDefaultExportFromCjs(logExports);

	log("esm-lib eval");
	const named = "n";
	var esmLib = "d";

	var esmLib$1 = /*#__PURE__*/Object.freeze({
		__proto__: null,
		default: esmLib,
		named: named
	});

	var require$$1 = /*@__PURE__*/getAugmentedNamespace(esmLib$1);

	var lazyInner = {};

	var hasRequiredLazyInner;

	function requireLazyInner () {
		if (hasRequiredLazyInner) return lazyInner;
		hasRequiredLazyInner = 1;
		requireLog()("lazy-inner eval");
		lazyInner.value = 42;
		return lazyInner;
	}

	var hasRequiredRequireEsm;

	function requireRequireEsm () {
		if (hasRequiredRequireEsm) return requireEsm$1;
		hasRequiredRequireEsm = 1;
		const log = requireLog();
		const lib = require$$1;
		log("lib " + typeof lib + " " + (typeof lib === "object" ? Object.keys(lib).sort().join(",") : lib));
		function late() {
		  return requireLazyInner().value;
		}
		log("late " + late());
		return requireEsm$1;
	}

	var requireEsmExports = requireRequireEsm();
	var requireEsm = /*@__PURE__*/getDefaultExportFromCjs(requireEsmExports);

	return requireEsm;

})();
