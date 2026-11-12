var app = (function () {
	'use strict';

	var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

	function createCommonjsModule(fn) {
	  var module = { exports: {} };
		return fn(module, module.exports), module.exports;
	}

	function commonjsRequire (target) {
		throw new Error('Could not dynamically require "' + target + '". Please configure the dynamicRequireTargets option of @rollup/plugin-commonjs appropriately for this require call to behave properly.');
	}

	var count = 0;
	var log = function log(line) {
	  count += 1;
	  console.log(count + " " + line);
	};

	var a = cycleA;

	log("b start");

	var name$2 = "b";
	var sawA = String(a.name) + "/" + String(a.fromB);
	log("b end");

	var cycleB = {
		name: name$2,
		sawA: sawA
	};

	log("a start");
	var name$1 = "a";

	var fromB = cycleB.name + ":" + cycleB.sawA;
	log("a end");

	var cycleA = {
		name: name$1,
		fromB: fromB
	};

	log("reassign eval");
	var reassign = function shout(text) {
	  return text + "!";
	};
	var extra = 7;
	reassign.extra = extra;

	log("props eval");
	var one = 1;
	var two = function () {
	  return 2;
	};

	var props = {
		one: one,
		two: two
	};

	var thisTop = createCommonjsModule(function (module, exports) {
	commonjsGlobal.viaThis = 5;
	exports.same = String(commonjsGlobal === exports) + ":" + String(exports.viaThis);
	});

	log("lazy-inner eval");
	var value = 42;

	var lazyInner = {
		value: value
	};

	var calls = 0;
	var lazyTwice = function () {
	  calls += 1;
	  return calls;
	};
	log("lazy-twice eval");

	log("index start");

	log("a.fromB " + a.fromB);

	log("reassign " + reassign("x") + " " + reassign.extra);

	log("props " + props.one + props.two());
	log("this " + thisTop.same);
	function later() {
	  return lazyInner.value;
	}
	log("before lazy");
	log("lazy " + later() + " " + later());
	function counter() {
	  return lazyTwice();
	}
	log("twice " + counter() + counter());
	const name = "./dyn-" + "target";
	try {
	  log("dynamic " + commonjsRequire(name).v);
	} catch (error) {
	  log("dynamic threw");
	}
	log("index end");

	var synthetic = {

	};

	return synthetic;

})();
