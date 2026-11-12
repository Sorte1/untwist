var app = (function () {
	'use strict';

	function commonjsRequire(path) {
		throw new Error('Could not dynamically require "' + path + '". Please configure the dynamicRequireTargets or/and ignoreDynamicRequires option of @rollup/plugin-commonjs appropriately for this require call to work.');
	}

	var synthetic = {};

	var count = 0;
	var log$3 = function log(line) {
	  count += 1;
	  console.log(count + " " + line);
	};

	var cycleA = {};

	var cycleB = {};

	const log$2 = log$3;
	log$2("b start");
	const a$1 = cycleA;
	cycleB.name = "b";
	cycleB.sawA = String(a$1.name) + "/" + String(a$1.fromB);
	log$2("b end");

	const log$1 = log$3;
	log$1("a start");
	cycleA.name = "a";
	const b = cycleB;
	cycleA.fromB = b.name + ":" + b.sawA;
	log$1("a end");

	var reassign = {exports: {}};

	log$3("reassign eval");
	reassign.exports = function shout(text) {
	  return text + "!";
	};
	reassign.exports.extra = 7;

	var reassignExports = reassign.exports;

	var props$1 = {};

	log$3("props eval");
	props$1.one = 1;
	props$1.two = function () {
	  return 2;
	};

	var thisTop = {};

	(function (exports) {
		thisTop.viaThis = 5;
		exports.same = String(thisTop === exports) + ":" + String(exports.viaThis); 
	} (thisTop));

	var lazyInner = {};

	log$3("lazy-inner eval");
	lazyInner.value = 42;

	var calls = 0;
	var lazyTwice = function () {
	  calls += 1;
	  return calls;
	};
	log$3("lazy-twice eval");

	const log = log$3;
	log("index start");
	const a = cycleA;
	log("a.fromB " + a.fromB);
	const shout = reassignExports;
	log("reassign " + shout("x") + " " + shout.extra);
	const props = props$1;
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

	return synthetic;

})();
