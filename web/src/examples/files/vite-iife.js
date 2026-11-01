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
		for (var name in all) __defProp(target, name, {
			get: all[name],
			enumerable: true
		});
		if (!no_symbols) __defProp(target, Symbol.toStringTag, { value: "Module" });
		return target;
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
	var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(isNodeMode || !mod || !mod.__esModule || !__hasOwnProp.call(mod, "default") ? __defProp(target, "default", {
		value: mod,
		enumerable: true
	}) : target, mod));
	// app/say.js
	function say(line) {
		console.log(line);
	}
	var init_say = __esmMin((() => {
		say("eval say");
	}));
	// app/greet.js
	init_say();
	say("eval greet");
	function greet(name) {
		return "hello " + name;
	}
	function shout(name) {
		return name.toUpperCase() + "!";
	}
	// app/math.js
	var math_exports = /* @__PURE__ */ __exportAll({
		add: () => add,
		mul: () => mul,
		triple: () => triple
	});
	init_say();
	say("eval math");
	var add = (a, b) => a + b;
	var mul = (a, b) => a * b;
	function triple(n) {
		return mul(n, 3);
	}
	// app/live.js
	init_say();
	say("eval live");
	var counter = 0;
	function bump() {
		counter += 1;
	}
	// app/reexport.js
	init_say();
	say("eval reexport");
	// app/data.json
	var data_exports = /* @__PURE__ */ __exportAll({
		default: () => data_default,
		items: () => items,
		name: () => name
	});
	var name, items, data_default;
	var init_data = __esmMin((() => {
		name = "fixture";
		items = [
			1,
			2,
			3
		];
		data_default = {
			name,
			items
		};
	}));
	// app/cjs.js
	var require_cjs = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var path = require("path");
		console.log("eval cjs");
		exports.join = function(a, b) {
			return path.posix.join(a, b);
		};
		exports.tag = "cjs-" + typeof module;
	}));
	// app/cycle-b.js
	var require_cycle_b = /* @__PURE__ */ __commonJSMin(((exports) => {
		console.log("eval cycle-b");
		var a = require_cycle_a();
		exports.seen = "b saw " + a.early + "," + a.late;
		exports.check = function() {
			return "b later " + a.late;
		};
	}));
	// app/cycle-a.js
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
	// app/lazy.js
	var lazy_exports = /* @__PURE__ */ __exportAll({
		default: () => lazy_default,
		value: () => 42
	}), lazy_default;
	var init_lazy = __esmMin((() => {
		init_say();
		say("eval lazy");
		lazy_default = "lazy-default";
	}));
	// app/index.js
	init_say();
	init_data();
	var import_cjs = /* @__PURE__ */ __toESM(require_cjs());
	var import_cycle_a = /* @__PURE__ */ __toESM(require_cycle_a());
	say("eval index");
	say("greet " + greet("ada") + " " + shout("bob"));
	say("math " + Object.keys(math_exports).sort().join(",") + " " + add(2, 3));
	say("live " + counter);
	bump();
	say("live " + counter);
	say("reexport " + shout("x") + " " + triple(4));
	say("json " + data_default.name + " " + data_default.items.length);
	say("cjs " + import_cjs.join("a", "b") + " " + import_cjs.tag);
	say("cycle " + import_cycle_a.report());
	Promise.resolve().then(() => (init_data(), data_exports)).then((lazy) => say("json lazy " + (lazy.default || lazy).name));
	Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => say("lazy " + lazy.value + " " + lazy.default));
})();
