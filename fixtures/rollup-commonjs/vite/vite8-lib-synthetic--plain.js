(function() {
	// \0rolldown/runtime.js
	var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);
	// apps/synthetic/log.js
	var require_log = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var count = 0;
		module.exports = function log(line) {
			count += 1;
			console.log(count + " " + line);
		};
	}));
	// apps/synthetic/cycle-b.js
	var require_cycle_b = /* @__PURE__ */ __commonJSMin(((exports) => {
		var log = require_log();
		log("b start");
		var a = require_cycle_a();
		exports.name = "b";
		exports.sawA = String(a.name) + "/" + String(a.fromB);
		log("b end");
	}));
	// apps/synthetic/cycle-a.js
	var require_cycle_a = /* @__PURE__ */ __commonJSMin(((exports) => {
		var log = require_log();
		log("a start");
		exports.name = "a";
		var b = require_cycle_b();
		exports.fromB = b.name + ":" + b.sawA;
		log("a end");
	}));
	// apps/synthetic/reassign.js
	var require_reassign = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		require_log()("reassign eval");
		module.exports = function shout(text) {
			return text + "!";
		};
		module.exports.extra = 7;
	}));
	// apps/synthetic/props.js
	var require_props = /* @__PURE__ */ __commonJSMin(((exports) => {
		require_log()("props eval");
		exports.one = 1;
		exports.two = function() {
			return 2;
		};
	}));
	// apps/synthetic/this-top.js
	var require_this_top = /* @__PURE__ */ __commonJSMin(((exports, module, this$1) => {
		this$1.viaThis = 5;
		exports.same = String(this$1 === exports) + ":" + String(exports.viaThis);
	}));
	// apps/synthetic/lazy-inner.js
	var require_lazy_inner = /* @__PURE__ */ __commonJSMin(((exports) => {
		require_log()("lazy-inner eval");
		exports.value = 42;
	}));
	// apps/synthetic/lazy-twice.js
	var require_lazy_twice = /* @__PURE__ */ __commonJSMin(((exports, module) => {
		var calls = 0;
		module.exports = function() {
			calls += 1;
			return calls;
		};
		require_log()("lazy-twice eval");
	}));
	// apps/synthetic/index.js
	var log = require_log();
	log("index start");
	log("a.fromB " + require_cycle_a().fromB);
	var shout = require_reassign();
	log("reassign " + shout("x") + " " + shout.extra);
	var props = require_props();
	log("props " + props.one + props.two());
	log("this " + require_this_top().same);
	function later() {
		return require_lazy_inner().value;
	}
	log("before lazy");
	log("lazy " + later() + " " + later());
	function counter() {
		return require_lazy_twice()();
	}
	log("twice " + counter() + counter());
	var name = "./dyn-target";
	try {
		log("dynamic " + require(name).v);
	} catch (error) {
		log("dynamic threw");
	}
	log("index end");
})();
