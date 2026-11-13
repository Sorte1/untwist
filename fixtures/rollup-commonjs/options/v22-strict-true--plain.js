var app = (function () {
	'use strict';

	var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

	function commonjsRequire(path) {
		throw new Error('Could not dynamically require "' + path + '". Please configure the dynamicRequireTargets or/and ignoreDynamicRequires option of @rollup/plugin-commonjs appropriately for this require call to work.');
	}

	var synthetic = {};

	var log;
	var hasRequiredLog;

	function requireLog () {
		if (hasRequiredLog) return log;
		hasRequiredLog = 1;
		var count = 0;
		log = function log(line) {
		  count += 1;
		  console.log(count + " " + line);
		};
		return log;
	}

	var cycleA = {};

	var cycleB = {};

	var hasRequiredCycleB;

	function requireCycleB () {
		if (hasRequiredCycleB) return cycleB;
		hasRequiredCycleB = 1;
		const log = requireLog();
		log("b start");
		const a = requireCycleA();
		cycleB.name = "b";
		cycleB.sawA = String(a.name) + "/" + String(a.fromB);
		log("b end");
		return cycleB;
	}

	var hasRequiredCycleA;

	function requireCycleA () {
		if (hasRequiredCycleA) return cycleA;
		hasRequiredCycleA = 1;
		const log = requireLog();
		log("a start");
		cycleA.name = "a";
		const b = requireCycleB();
		cycleA.fromB = b.name + ":" + b.sawA;
		log("a end");
		return cycleA;
	}

	var reassign = {exports: {}};

	var hasRequiredReassign;

	function requireReassign () {
		if (hasRequiredReassign) return reassign.exports;
		hasRequiredReassign = 1;
		requireLog()("reassign eval");
		reassign.exports = function shout(text) {
		  return text + "!";
		};
		reassign.exports.extra = 7;
		return reassign.exports;
	}

	var props = {};

	var hasRequiredProps;

	function requireProps () {
		if (hasRequiredProps) return props;
		hasRequiredProps = 1;
		requireLog()("props eval");
		props.one = 1;
		props.two = function () {
		  return 2;
		};
		return props;
	}

	var thisTop = {};

	var hasRequiredThisTop;

	function requireThisTop () {
		if (hasRequiredThisTop) return thisTop;
		hasRequiredThisTop = 1;
		(function (exports) {
			commonjsGlobal.viaThis = 5;
			exports.same = String(commonjsGlobal === exports) + ":" + String(exports.viaThis);
	} (thisTop));
		return thisTop;
	}

	var lazyInner = {};

	var hasRequiredLazyInner;

	function requireLazyInner () {
		if (hasRequiredLazyInner) return lazyInner;
		hasRequiredLazyInner = 1;
		requireLog()("lazy-inner eval");
		lazyInner.value = 42;
		return lazyInner;
	}

	var lazyTwice;
	var hasRequiredLazyTwice;

	function requireLazyTwice () {
		if (hasRequiredLazyTwice) return lazyTwice;
		hasRequiredLazyTwice = 1;
		var calls = 0;
		lazyTwice = function () {
		  calls += 1;
		  return calls;
		};
		requireLog()("lazy-twice eval");
		return lazyTwice;
	}

	var hasRequiredSynthetic;

	function requireSynthetic () {
		if (hasRequiredSynthetic) return synthetic;
		hasRequiredSynthetic = 1;
		const log = requireLog();
		log("index start");
		const a = requireCycleA();
		log("a.fromB " + a.fromB);
		const shout = requireReassign();
		log("reassign " + shout("x") + " " + shout.extra);
		const props = requireProps();
		log("props " + props.one + props.two());
		log("this " + requireThisTop().same);
		function later() {
		  return requireLazyInner().value;
		}
		log("before lazy");
		log("lazy " + later() + " " + later());
		function counter() {
		  return requireLazyTwice()();
		}
		log("twice " + counter() + counter());
		const name = "./dyn-" + "target";
		try {
		  log("dynamic " + commonjsRequire(name).v);
		} catch (error) {
		  log("dynamic threw");
		}
		log("index end");
		return synthetic;
	}

	var syntheticExports = requireSynthetic();

	return syntheticExports;

})();
