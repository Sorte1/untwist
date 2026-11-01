(() => {
  var __create = Object.create;
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __getProtoOf = Object.getPrototypeOf;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __require = /* @__PURE__ */ ((x) => typeof require !== "undefined" ? require : typeof Proxy !== "undefined" ? new Proxy(x, {
    get: (a, b) => (typeof require !== "undefined" ? require : a)[b]
  }) : x)(function(x) {
    if (typeof require !== "undefined") return require.apply(this, arguments);
    throw Error('Dynamic require of "' + x + '" is not supported');
  });
  var __esm = (fn, res, err) => function __init() {
    if (err) throw err[0];
    try {
      return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
    } catch (e) {
      throw err = [e], e;
    }
  };
  var __commonJS = (cb, mod) => function __require2() {
    try {
      return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
    } catch (e) {
      throw mod = 0, e;
    }
  };
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
    // If the importer is in node compatibility mode or this is not an ESM
    // file that has been converted to a CommonJS file using a Babel-
    // compatible transform (i.e. "__esModule" has not been set), then set
    // "default" to the CommonJS "module.exports" for node compatibility.
    isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
    mod
  ));

  // app/say.js
  function say(line) {
    console.log(line);
  }
  var init_say = __esm({
    "app/say.js"() {
      say("eval say");
    }
  });

  // app/data.json
  var require_data = __commonJS({
    "app/data.json"(exports, module) {
      module.exports = { name: "fixture", items: [1, 2, 3] };
    }
  });

  // app/cjs.js
  var require_cjs = __commonJS({
    "app/cjs.js"(exports, module) {
      var path = __require("path");
      console.log("eval cjs");
      exports.join = function(a, b) {
        return path.posix.join(a, b);
      };
      exports.tag = "cjs-" + typeof module;
    }
  });

  // app/cycle-b.js
  var require_cycle_b = __commonJS({
    "app/cycle-b.js"(exports) {
      console.log("eval cycle-b");
      var a = require_cycle_a();
      exports.seen = "b saw " + a.early + "," + a.late;
      exports.check = function() {
        return "b later " + a.late;
      };
    }
  });

  // app/cycle-a.js
  var require_cycle_a = __commonJS({
    "app/cycle-a.js"(exports) {
      console.log("eval cycle-a start");
      exports.early = "a-early";
      var b = require_cycle_b();
      exports.late = "a-late";
      exports.report = function() {
        return b.seen + " " + b.check();
      };
      console.log("eval cycle-a end");
    }
  });

  // app/lazy.js
  var lazy_exports = {};
  __export(lazy_exports, {
    default: () => lazy_default,
    value: () => value
  });
  var value, lazy_default;
  var init_lazy = __esm({
    "app/lazy.js"() {
      init_say();
      say("eval lazy");
      value = 42;
      lazy_default = "lazy-default";
    }
  });

  // app/index.js
  init_say();

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
  var math_exports = {};
  __export(math_exports, {
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

  // app/index.js
  var import_data = __toESM(require_data());
  var import_cjs = __toESM(require_cjs());
  var import_cycle_a = __toESM(require_cycle_a());
  say("eval index");
  say("greet " + greet("ada") + " " + shout("bob"));
  say("math " + Object.keys(math_exports).sort().join(",") + " " + add(2, 3));
  say("live " + counter);
  bump();
  say("live " + counter);
  say("reexport " + shout("x") + " " + triple(4));
  say("json " + import_data.default.name + " " + import_data.default.items.length);
  say("cjs " + import_cjs.default.join("a", "b") + " " + import_cjs.default.tag);
  say("cycle " + import_cycle_a.default.report());
  Promise.resolve().then(() => __toESM(require_data())).then((lazy) => say("json lazy " + (lazy.default || lazy).name));
  Promise.resolve().then(() => (init_lazy(), lazy_exports)).then((lazy) => say("lazy " + lazy.value + " " + lazy.default));
})();
