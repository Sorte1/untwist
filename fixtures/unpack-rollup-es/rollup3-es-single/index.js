import require$$0$1 from 'node:path';

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

var greet$1 = {exports: {}};

var hasRequiredGreet;

function requireGreet () {
	if (hasRequiredGreet) return greet$1.exports;
	hasRequiredGreet = 1;
	console.log("eval greet");
	const path = require$$0$1;
	function greet(name) {
	  return "hello " + path.basename(name);
	}
	greet.shout = function (name) {
	  return greet(name).toUpperCase();
	};
	greet$1.exports = greet;
	greet$1.exports.shout = greet.shout;
	return greet$1.exports;
}

var greetExports = requireGreet();
var greet = /*@__PURE__*/getDefaultExportFromCjs(greetExports);

var name = "fixture";
var items$1 = [
	1,
	2,
	3,
	4,
	5
];
var data = {
	name: name,
	items: items$1
};

var cycleA = {};

var cycleB = {};

var hasRequiredCycleB;

function requireCycleB () {
	if (hasRequiredCycleB) return cycleB;
	hasRequiredCycleB = 1;
	console.log("eval cycle-b");
	var a = requireCycleA();
	cycleB.seen = "b saw " + a.early + "," + a.late;
	cycleB.check = function () {
	  return "b later " + a.late;
	};
	return cycleB;
}

var hasRequiredCycleA;

function requireCycleA () {
	if (hasRequiredCycleA) return cycleA;
	hasRequiredCycleA = 1;
	console.log("eval cycle-a start");
	cycleA.early = "a-early";
	var b = requireCycleB();
	cycleA.late = "a-late";
	cycleA.report = function () {
	  return b.seen + " " + b.check();
	};
	console.log("eval cycle-a end");
	return cycleA;
}

var cycleAExports = requireCycleA();

console.log("eval util");
const label = (name) => `[${name.padStart(8, ".")}]`;
function total(list) {
  return list.reduce((sum, n) => sum + n, 0);
}
class Counter {
  constructor() {
    this.value = 0;
  }
  add(n) {
    this.value += n;
  }
}

var math = {};

var hasRequiredMath;

function requireMath () {
	if (hasRequiredMath) return math;
	hasRequiredMath = 1;
	console.log("eval math");
	math.twice = function (n) {
	  return n * 2;
	};
	math.half = (n) => n / 2;
	return math;
}

var mathExports = requireMath();

const own = "own";

var barrel = /*#__PURE__*/_mergeNamespaces({
	__proto__: null,
	own: own
}, [mathExports]);

var bridge = {};

console.log("eval shape");
const kind = "square";
function area(side) {
  return side * side;
}
var shape = "shape-default";

var shape$1 = /*#__PURE__*/Object.freeze({
	__proto__: null,
	area: area,
	default: shape,
	kind: kind
});

var require$$0 = /*@__PURE__*/getAugmentedNamespace(shape$1);

var hasRequiredBridge;

function requireBridge () {
	if (hasRequiredBridge) return bridge;
	hasRequiredBridge = 1;
	console.log("eval bridge");
	const shape = require$$0;
	bridge.describe = function () {
	  return shape.kind + " " + shape.area(3) + " " + shape.default;
	};
	return bridge;
}

var bridgeExports = requireBridge();

const items = data.items;
console.log("eval index");
console.log("greet " + greet("ada") + " " + greetExports.shout("bob"));
console.log("cycle " + cycleAExports.report());
console.log("label " + label(data.name) + " total " + total(items));
console.log("barrel " + own + " " + mathExports.twice(4) + " " + Object.keys(barrel).sort().join(","));
console.log("bridge " + bridgeExports.describe());
const counter = new Counter();
counter.add(2);
console.log("counter " + counter.value + " " + Counter.name);
Promise.resolve().then(function () { return lazy$1; }).then((lazy) => {
  console.log("lazy " + lazy.value + " " + lazy.default);
});

console.log("eval lazy");
const value = 42;
var lazy = "lazy-default";

var lazy$1 = /*#__PURE__*/Object.freeze({
	__proto__: null,
	default: lazy,
	value: value
});

export { Counter, label, total };
