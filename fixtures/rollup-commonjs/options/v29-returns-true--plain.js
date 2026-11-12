var app = (function () {
	'use strict';

	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
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
	var require$$1 = "d";

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
		log("lib " + typeof lib + " " + (lib));
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
