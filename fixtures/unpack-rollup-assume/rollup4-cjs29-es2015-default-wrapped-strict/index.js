(function () {
  "use strict";
  var hostConfig;
  var hostHook;
(function () {
	'use strict';

	var _documentCurrentScript = typeof document !== 'undefined' ? document.currentScript : null;
	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
	}

	/**
	 * Helpers.
	 */

	var ms$1;
	var hasRequiredMs;

	function requireMs () {
		if (hasRequiredMs) return ms$1;
		hasRequiredMs = 1;
		var s = 1000;
		var m = s * 60;
		var h = m * 60;
		var d = h * 24;
		var w = d * 7;
		var y = d * 365.25;

		/**
		 * Parse or format the given `val`.
		 *
		 * Options:
		 *
		 *  - `long` verbose formatting [false]
		 *
		 * @param {String|Number} val
		 * @param {Object} [options]
		 * @throws {Error} throw an error if val is not a non-empty string or a number
		 * @return {String|Number}
		 * @api public
		 */

		ms$1 = function (val, options) {
		  options = options || {};
		  var type = typeof val;
		  if (type === 'string' && val.length > 0) {
		    return parse(val);
		  } else if (type === 'number' && isFinite(val)) {
		    return options.long ? fmtLong(val) : fmtShort(val);
		  }
		  throw new Error(
		    'val is not a non-empty string or a valid number. val=' +
		      JSON.stringify(val)
		  );
		};

		/**
		 * Parse the given `str` and return milliseconds.
		 *
		 * @param {String} str
		 * @return {Number}
		 * @api private
		 */

		function parse(str) {
		  str = String(str);
		  if (str.length > 100) {
		    return;
		  }
		  var match = /^(-?(?:\d+)?\.?\d+) *(milliseconds?|msecs?|ms|seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|h|days?|d|weeks?|w|years?|yrs?|y)?$/i.exec(
		    str
		  );
		  if (!match) {
		    return;
		  }
		  var n = parseFloat(match[1]);
		  var type = (match[2] || 'ms').toLowerCase();
		  switch (type) {
		    case 'years':
		    case 'year':
		    case 'yrs':
		    case 'yr':
		    case 'y':
		      return n * y;
		    case 'weeks':
		    case 'week':
		    case 'w':
		      return n * w;
		    case 'days':
		    case 'day':
		    case 'd':
		      return n * d;
		    case 'hours':
		    case 'hour':
		    case 'hrs':
		    case 'hr':
		    case 'h':
		      return n * h;
		    case 'minutes':
		    case 'minute':
		    case 'mins':
		    case 'min':
		    case 'm':
		      return n * m;
		    case 'seconds':
		    case 'second':
		    case 'secs':
		    case 'sec':
		    case 's':
		      return n * s;
		    case 'milliseconds':
		    case 'millisecond':
		    case 'msecs':
		    case 'msec':
		    case 'ms':
		      return n;
		    default:
		      return undefined;
		  }
		}

		/**
		 * Short format for `ms`.
		 *
		 * @param {Number} ms
		 * @return {String}
		 * @api private
		 */

		function fmtShort(ms) {
		  var msAbs = Math.abs(ms);
		  if (msAbs >= d) {
		    return Math.round(ms / d) + 'd';
		  }
		  if (msAbs >= h) {
		    return Math.round(ms / h) + 'h';
		  }
		  if (msAbs >= m) {
		    return Math.round(ms / m) + 'm';
		  }
		  if (msAbs >= s) {
		    return Math.round(ms / s) + 's';
		  }
		  return ms + 'ms';
		}

		/**
		 * Long format for `ms`.
		 *
		 * @param {Number} ms
		 * @return {String}
		 * @api private
		 */

		function fmtLong(ms) {
		  var msAbs = Math.abs(ms);
		  if (msAbs >= d) {
		    return plural(ms, msAbs, d, 'day');
		  }
		  if (msAbs >= h) {
		    return plural(ms, msAbs, h, 'hour');
		  }
		  if (msAbs >= m) {
		    return plural(ms, msAbs, m, 'minute');
		  }
		  if (msAbs >= s) {
		    return plural(ms, msAbs, s, 'second');
		  }
		  return ms + ' ms';
		}

		/**
		 * Pluralization helper.
		 */

		function plural(ms, msAbs, n, name) {
		  var isPlural = msAbs >= n * 1.5;
		  return Math.round(ms / n) + ' ' + name + (isPlural ? 's' : '');
		}
		return ms$1;
	}

	var msExports = requireMs();
	const ms = /*@__PURE__*/getDefaultExportFromCjs(msExports);

	/*
	object-assign
	(c) Sindre Sorhus
	@license MIT
	*/

	var objectAssign;
	var hasRequiredObjectAssign;

	function requireObjectAssign () {
		if (hasRequiredObjectAssign) return objectAssign;
		hasRequiredObjectAssign = 1;
		/* eslint-disable no-unused-vars */
		var getOwnPropertySymbols = Object.getOwnPropertySymbols;
		var hasOwnProperty = Object.prototype.hasOwnProperty;
		var propIsEnumerable = Object.prototype.propertyIsEnumerable;

		function toObject(val) {
			if (val === null || val === undefined) {
				throw new TypeError('Object.assign cannot be called with null or undefined');
			}

			return Object(val);
		}

		function shouldUseNative() {
			try {
				if (!Object.assign) {
					return false;
				}

				// Detect buggy property enumeration order in older V8 versions.

				// https://bugs.chromium.org/p/v8/issues/detail?id=4118
				var test1 = new String('abc');  // eslint-disable-line no-new-wrappers
				test1[5] = 'de';
				if (Object.getOwnPropertyNames(test1)[0] === '5') {
					return false;
				}

				// https://bugs.chromium.org/p/v8/issues/detail?id=3056
				var test2 = {};
				for (var i = 0; i < 10; i++) {
					test2['_' + String.fromCharCode(i)] = i;
				}
				var order2 = Object.getOwnPropertyNames(test2).map(function (n) {
					return test2[n];
				});
				if (order2.join('') !== '0123456789') {
					return false;
				}

				// https://bugs.chromium.org/p/v8/issues/detail?id=3056
				var test3 = {};
				'abcdefghijklmnopqrst'.split('').forEach(function (letter) {
					test3[letter] = letter;
				});
				if (Object.keys(Object.assign({}, test3)).join('') !==
						'abcdefghijklmnopqrst') {
					return false;
				}

				return true;
			} catch (err) {
				// We don't expect any of the above to throw, but better to be safe.
				return false;
			}
		}

		objectAssign = shouldUseNative() ? Object.assign : function (target, source) {
			var from;
			var to = toObject(target);
			var symbols;

			for (var s = 1; s < arguments.length; s++) {
				from = Object(arguments[s]);

				for (var key in from) {
					if (hasOwnProperty.call(from, key)) {
						to[key] = from[key];
					}
				}

				if (getOwnPropertySymbols) {
					symbols = getOwnPropertySymbols(from);
					for (var i = 0; i < symbols.length; i++) {
						if (propIsEnumerable.call(from, symbols[i])) {
							to[symbols[i]] = from[symbols[i]];
						}
					}
				}
			}

			return to;
		};
		return objectAssign;
	}

	var objectAssignExports = requireObjectAssign();
	const assign = /*@__PURE__*/getDefaultExportFromCjs(objectAssignExports);

	console.log("state evaluated");
	let counter = 0;
	function bump() {
	  counter += 1;
	}

	console.log("host evaluated", typeof hostConfig);
	function configured() {
	  return typeof hostConfig !== "undefined" && hostConfig !== null ? JSON.stringify(hostConfig) : "none";
	}
	function hooked(n) {
	  return typeof hostHook === "function" ? hostHook(n) : "none";
	}

	var legacy$1 = {};

	var helper;
	var hasRequiredHelper;

	function requireHelper () {
		if (hasRequiredHelper) return helper;
		hasRequiredHelper = 1;
		console.log("helper evaluated");
		helper = {
		  scale: function (n, k) {
		    return n * k;
		  },
		};
		return helper;
	}

	var hasRequiredLegacy;

	function requireLegacy () {
		if (hasRequiredLegacy) return legacy$1;
		hasRequiredLegacy = 1;
		var helper = requireHelper();
		console.log("legacy evaluated");
		legacy$1.twice = function (n) {
		  return helper.scale(n, 2);
		};
		legacy$1.name = "legacy";
		return legacy$1;
	}

	var legacyExports = requireLegacy();
	const legacy = /*@__PURE__*/getDefaultExportFromCjs(legacyExports);

	var host$1 = {};

	var hasRequiredHost;

	function requireHost () {
		if (hasRequiredHost) return host$1;
		hasRequiredHost = 1;
		console.log("host-cjs evaluated", typeof hostConfig);
		host$1.mode = function () {
		  return typeof hostConfig !== "undefined" && hostConfig !== null ? hostConfig.mode : "none";
		};
		host$1.late = function (n) {
		  return typeof hostHook === "function" ? hostHook(n) : "none";
		};
		return host$1;
	}

	var hostExports = requireHost();
	const host = /*@__PURE__*/getDefaultExportFromCjs(hostExports);

	console.log("entry starts");
	const here = new URL((_documentCurrentScript && _documentCurrentScript.tagName.toUpperCase() === 'SCRIPT' && _documentCurrentScript.src || new URL('index.js', document.baseURI).href));
	console.log("scheme", here.protocol);
	console.log("file", here.pathname.split("/").pop());
	console.log("ms", ms(90000), ms("2h"));
	console.log("assign", JSON.stringify(assign({ a: 1 }, { b: 2 })));
	bump();
	bump();
	console.log("counter", counter);
	console.log("legacy", legacy.twice(21), legacy.name);
	console.log("config now", configured());
	console.log("hook now", hooked(1));
	console.log("host-cjs now", host.mode(), host.late(3));
	setTimeout(() => {
	  console.log("config later", configured());
	  console.log("hook later", hooked(2));
	  console.log("host-cjs later", host.mode(), host.late(4));
	}, 0);

})();
  hostConfig = { mode: "late" };
  hostHook = function (n) {
    return "hooked " + n;
  };
})();
