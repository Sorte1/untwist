(function () {
	'use strict';

	function _mergeNamespaces(n, m) {
		m.forEach(function (e) {
			e && typeof e !== 'string' && !Array.isArray(e) && Object.keys(e).forEach(function (k) {
				if (k !== 'default' && !(k in n)) {
					var d = Object.getOwnPropertyDescriptor(e, k);
					Object.defineProperty(n, k, d.get ? d : {
						enumerable: true,
						get: function () { return e[k]; }
					});
				}
			});
		});
		return Object.freeze(n);
	}

	function getDefaultExportFromCjs (x) {
		return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
	}

	var log$1 = {exports: {}};

	(function (module) {
		var count = 0;
		module.exports = function log(line) {
		  count += 1;
		  console.log(count + " " + line);
		};
	} (log$1));

	var log = log$1.exports.default;

	var reassign = {exports: {}};

	(function (module) {
		log$1.exports("reassign eval");
		module.exports = function shout(text) {
		  return text + "!";
		};
		module.exports.extra = 7;
	} (reassign));

	var shout = reassign.exports.default;

	var props$1 = {};

	log$1.exports("props eval");
	var one = props$1.one = 1;
	var two = props$1.two = function () {
	  return 2;
	};

	var props = /*#__PURE__*/_mergeNamespaces({
		__proto__: null,
		one: one,
		two: two,
		'default': props$1
	}, [props$1]);

	var cycleA = {};

	var cycleB = {};

	var hasRequiredCycleB;

	function requireCycleB () {
		if (hasRequiredCycleB) return cycleB;
		hasRequiredCycleB = 1;
		const log = log$1.exports;
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
		const log = log$1.exports;
		log("a start");
		cycleA.name = "a";
		const b = requireCycleB();
		cycleA.fromB = b.name + ":" + b.sawA;
		log("a end");
		return cycleA;
	}

	var cycleAExports = requireCycleA();
	var cycle = /*@__PURE__*/getDefaultExportFromCjs(cycleAExports);

	log("esm start");
	log("shout " + shout("y") + " " + reassign.exports.extra);
	log("props " + Object.keys(props).sort().join(",") + " " + one);
	log("cycle " + cycle.fromB);

})();
