(function() {

// \0rolldown/runtime.js
	var __create = Object.create;
	var __defProp = Object.defineProperty;
	var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
	var __getOwnPropNames = Object.getOwnPropertyNames;
	var __getProtoOf = Object.getPrototypeOf;
	var __hasOwnProp = Object.prototype.hasOwnProperty;
	var __esmMin = (fn, res, err) => () => {
		if (err) throw err[0];
		try {
			return fn && (res = fn(fn = 0)), res;
		} catch (e) {
			throw err = [e], e;
		}
	};
	var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
	var __exportAll = (all, no_symbols) => {
		let target = {};
		for (var name in all) {
			__defProp(target, name, {
				get: all[name],
				enumerable: true
			});
		}
		if (!no_symbols) {
			__defProp(target, Symbol.toStringTag, { value: "Module" });
		}
		return target;
	};
	var __copyProps = (to, from, except, desc) => {
		if (from && typeof from === "object" || typeof from === "function") {
			for (var keys = __getOwnPropNames(from), i = 0, n = keys.length, key; i < n; i++) {
				key = keys[i];
				if (!__hasOwnProp.call(to, key) && key !== except) {
					__defProp(to, key, {
						get: ((k) => from[k]).bind(null, key),
						enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable
					});
				}
			}
		}
		return to;
	};
	var __reExport = (target, mod, secondTarget) => (__copyProps(target, mod, "default"), secondTarget && __copyProps(secondTarget, mod, "default"));
	var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(isNodeMode || !mod || !mod.__esModule || !__hasOwnProp.call(mod, "default") ? __defProp(target, "default", {
		value: mod,
		enumerable: true
	}) : target, mod));
	var __toCommonJS = (mod) => __hasOwnProp.call(mod, "module.exports") ? mod["module.exports"] : __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// tools/rolldown/app/greet.cjs
	var require_greet = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		console.log("eval greet");
		const path = require("node:path");
		function greet(name) {
			return "hello " + path.basename(name);
		}
		greet.shout = function(name) {
			return greet(name).toUpperCase();
		};
		module.exports = greet;
		module.exports.shout = greet.shout;
	}));

// tools/rolldown/app/data.json
	var name = "fixture";
	var items$1 = [
		1,
		2,
		3,
		4,
		5
	];

// tools/rolldown/app/cycle-b.cjs
	var require_cycle_b = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cycle-b");
		var a = require_cycle_a();
		exports.seen = "b saw " + a.early + "," + a.late;
		exports.check = function() {
			return "b later " + a.late;
		};
	}));

// tools/rolldown/app/cycle-a.cjs
	var require_cycle_a = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cycle-a start");
		exports.early = "a-early";
		var b = require_cycle_b();
		exports.late = "a-late";
		exports.report = function() {
			return b.seen + " " + b.check();
		};
		console.log("eval cycle-a end");
	}));

// tools/rolldown/app/util.js
var import_greet = /* @__PURE__ */ __toESM(require_greet());
var import_cycle_a = require_cycle_a();
	console.log("eval util");
	const label = (name) => `[${name.padStart(8, ".")}]`;
	function total(list) {
		return list.reduce((sum, n) => sum + n, 0);
	}
	var Counter = class {
		constructor() {
			this.value = 0;
		}
		add(n) {
			this.value += n;
		}
	};

// tools/rolldown/app/math.cjs
	var require_math = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval math");
		exports.twice = function(n) {
			return n * 2;
		};
		exports.half = (n) => n / 2;
	}));

// tools/rolldown/app/barrel.js
	var barrel_exports = /* @__PURE__ */ __exportAll({ own: () => "own" });
	__reExport(barrel_exports, /* @__PURE__ */ __toESM(require_math()));
	const own = "own";

// tools/rolldown/app/shape.js
	var shape_exports = /* @__PURE__ */ __exportAll({
		area: () => area,
		default: () => shape_default,
		kind: () => kind
	});
	function area(side) {
		return side * side;
	}
	var kind, shape_default;
	var init_shape = __esmMin((() => {
		console.log("eval shape");
		kind = "square";
		shape_default = "shape-default";
	}));

// tools/rolldown/app/bridge.cjs
	var require_bridge = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval bridge");
		const shape = (init_shape(), __toCommonJS(shape_exports));
		exports.describe = function() {
			return shape.kind + " " + shape.area(3) + " " + shape.default;
		};
	}));

// tools/rolldown/app/lazy.js
	var lazy_exports = /* @__PURE__ */ __exportAll({
		default: () => lazy_default,
		value: () => 42
	});
	var value, lazy_default;
	var init_lazy = __esmMin((() => {
		console.log("eval lazy");
		value = 42;
		lazy_default = "lazy-default";
	}));

// tools/rolldown/app/index.js
	var import_bridge = require_bridge();
	const items = items$1;
	console.log("eval index");
	console.log("greet " + (0, import_greet.default)("ada") + " " + (0, import_greet.shout)("bob"));
	console.log("cycle " + (0, import_cycle_a.report)());
	console.log("label " + label(name) + " total " + total(items));
	console.log("barrel " + "own" + " " + barrel_exports.twice(4) + " " + Object.keys(barrel_exports).sort().join(","));
	console.log("bridge " + (0, import_bridge.describe)());
	const counter = new Counter();
	counter.add(2);
	console.log("counter " + counter.value + " " + Counter.name);
	Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => {
		console.log("lazy " + lazy.value + " " + lazy.default);
	});

})();