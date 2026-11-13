var app = (function () {
	'use strict';

	function getAugmentedNamespace(n) {
	  var f = n.default;
		if (typeof f == "function") {
			var a = function () {
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

	var requireEsm = {};

	var count = 0;
	var log$1 = function log(line) {
	  count += 1;
	  console.log(count + " " + line);
	};

	log$1("esm-lib eval");
	const named = "n";
	var esmLib = "d";

	var esmLib$1 = /*#__PURE__*/Object.freeze({
		__proto__: null,
		named: named,
		'default': esmLib
	});

	var require$$1 = /*@__PURE__*/getAugmentedNamespace(esmLib$1);

	var lazyInner = {};

	var hasRequiredLazyInner;

	function requireLazyInner () {
		if (hasRequiredLazyInner) return lazyInner;
		hasRequiredLazyInner = 1;
		log$1("lazy-inner eval");
		lazyInner.value = 42;
		return lazyInner;
	}

	const log = log$1;
	const lib = require$$1;
	log("lib " + typeof lib + " " + (typeof lib === "object" ? Object.keys(lib).sort().join(",") : lib));
	function late() {
	  return requireLazyInner().value;
	}
	log("late " + late());

	return requireEsm;

})();
