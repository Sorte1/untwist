import { createRequire } from "node:module";

//#region rolldown:runtime
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __esm = (fn, res) => function() {
	return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __commonJS = (cb, mod) => function() {
	return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __export = (target, all) => {
	for (var name$1 in all) __defProp(target, name$1, {
		get: all[name$1],
		enumerable: true
	});
};
var __copyProps = (to, from, except, desc) => {
	if (from && typeof from === "object" || typeof from === "function") for (var keys = __getOwnPropNames(from), i = 0, n = keys.length, key; i < n; i++) {
		key = keys[i];
		if (!__hasOwnProp.call(to, key) && key !== except) __defProp(to, key, {
			get: ((k) => from[k]).bind(null, key),
			enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable
		});
	}
	return to;
};
var __reExport = (target, mod, secondTarget) => (__copyProps(target, mod, "default"), secondTarget && __copyProps(secondTarget, mod, "default"));
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", {
	value: mod,
	enumerable: true
}) : target, mod));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var __require = /* @__PURE__ */ createRequire(import.meta.url);

//#endregion
//#region tools/rolldown/app/greet.cjs
var require_greet = /* @__PURE__ */ __commonJS({ "tools/rolldown/app/greet.cjs": ((exports, module) => {
	console.log("eval greet");
	const path = __require("node:path");
	function greet$1(name$1) {
		return "hello " + path.basename(name$1);
	}
	greet$1.shout = function(name$1) {
		return greet$1(name$1).toUpperCase();
	};
	module.exports = greet$1;
	module.exports.shout = greet$1.shout;
}) });

//#endregion
//#region tools/rolldown/app/data.json
var name = "fixture";
var items$1 = [
	1,
	2,
	3,
	4,
	5
];
var data_default = {
	name,
	items: items$1
};

//#endregion
//#region tools/rolldown/app/cycle-b.cjs
var require_cycle_b = /* @__PURE__ */ __commonJS({ "tools/rolldown/app/cycle-b.cjs": ((exports) => {
	console.log("eval cycle-b");
	var a = require_cycle_a();
	exports.seen = "b saw " + a.early + "," + a.late;
	exports.check = function() {
		return "b later " + a.late;
	};
}) });

//#endregion
//#region tools/rolldown/app/cycle-a.cjs
var require_cycle_a = /* @__PURE__ */ __commonJS({ "tools/rolldown/app/cycle-a.cjs": ((exports) => {
	console.log("eval cycle-a start");
	exports.early = "a-early";
	var b = require_cycle_b();
	exports.late = "a-late";
	exports.report = function() {
		return b.seen + " " + b.check();
	};
	console.log("eval cycle-a end");
}) });

//#endregion
//#region tools/rolldown/app/util.js
console.log("eval util");
const label = (name$1) => `[${name$1.padStart(8, ".")}]`;
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

//#endregion
//#region tools/rolldown/app/math.cjs
var require_math = /* @__PURE__ */ __commonJS({ "tools/rolldown/app/math.cjs": ((exports) => {
	console.log("eval math");
	exports.twice = function(n) {
		return n * 2;
	};
	exports.half = (n) => n / 2;
}) });

//#endregion
//#region tools/rolldown/app/barrel.js
var barrel_exports = {};
__export(barrel_exports, { own: () => own });
__reExport(barrel_exports, /* @__PURE__ */ __toESM(require_math()));
const own = "own";

//#endregion
//#region tools/rolldown/app/shape.js
var shape_exports = {};
__export(shape_exports, {
	area: () => area,
	default: () => shape_default,
	kind: () => kind
});
function area(side) {
	return side * side;
}
var kind, shape_default;
var init_shape = __esm({ "tools/rolldown/app/shape.js": (() => {
	console.log("eval shape");
	kind = "square";
	shape_default = "shape-default";
}) });

//#endregion
//#region tools/rolldown/app/bridge.cjs
var require_bridge = /* @__PURE__ */ __commonJS({ "tools/rolldown/app/bridge.cjs": ((exports) => {
	console.log("eval bridge");
	const shape = (init_shape(), __toCommonJS(shape_exports));
	exports.describe = function() {
		return shape.kind + " " + shape.area(3) + " " + shape.default;
	};
}) });

//#endregion
//#region tools/rolldown/app/index.js
var import_greet = /* @__PURE__ */ __toESM(require_greet());
var import_cycle_a = /* @__PURE__ */ __toESM(require_cycle_a());
var import_bridge = /* @__PURE__ */ __toESM(require_bridge());
const items = data_default.items;
console.log("eval index");
console.log("greet " + (0, import_greet.default)("ada") + " " + (0, import_greet.shout)("bob"));
console.log("cycle " + (0, import_cycle_a.report)());
console.log("label " + label(data_default.name) + " total " + total(items));
console.log("barrel " + own + " " + barrel_exports.twice(4) + " " + Object.keys(barrel_exports).sort().join(","));
console.log("bridge " + (0, import_bridge.describe)());
const counter = new Counter();
counter.add(2);
console.log("counter " + counter.value + " " + Counter.name);
import("./lazy.chunk.js").then((lazy) => {
	console.log("lazy " + lazy.value + " " + lazy.default);
});

//#endregion