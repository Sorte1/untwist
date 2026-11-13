var app = (function () {
	'use strict';

	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
	}

	var props$1 = {};

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

	var hasRequiredProps;

	function requireProps () {
		if (hasRequiredProps) return props$1;
		hasRequiredProps = 1;
		requireLog()("props eval");
		props$1.one = 1;
		props$1.two = function () {
		  return 2;
		};
		return props$1;
	}

	var logExports = requireLog();
	var log = /*@__PURE__*/getDefaultExportFromCjs(logExports);

	const props = requireProps();
	log("mixed " + props.one);
	var mixed = "mixed";

	return mixed;

})();
