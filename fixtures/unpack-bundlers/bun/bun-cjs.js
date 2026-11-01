// @bun @bun-cjs
(function(exports, require, module, __filename, __dirname) {var __create = Object.create;
var __getProtoOf = Object.getPrototypeOf;
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
function __accessProp(key) {
  return this[key];
}
var __toESMCache_node;
var __toESMCache_esm;
var __toESM = (mod, isNodeMode, target) => {
  var canCache = mod != null && typeof mod === "object";
  if (canCache) {
    var cache = isNodeMode ? __toESMCache_node ??= new WeakMap : __toESMCache_esm ??= new WeakMap;
    var cached = cache.get(mod);
    if (cached)
      return cached;
  }
  target = mod != null ? __create(__getProtoOf(mod)) : {};
  const to = isNodeMode || !mod || !mod.__esModule || !__hasOwnProp.call(mod, "default") ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
  if (mod && typeof mod === "object" || typeof mod === "function") {
    for (let key of __getOwnPropNames(mod))
      if (!__hasOwnProp.call(to, key))
        __defProp(to, key, {
          get: __accessProp.bind(mod, key),
          enumerable: true
        });
  }
  if (canCache)
    cache.set(mod, to);
  return to;
};
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __returnValue = (v) => v;
function __exportSetter(name, newValue) {
  this[name] = __returnValue.bind(null, newValue);
}
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, {
      get: all[name],
      enumerable: true,
      configurable: true,
      set: __exportSetter.bind(all, name)
    });
};
var __esm = (fn, res, err) => () => {
  if (fn)
    try {
      res = fn(fn = 0);
    } catch (e) {
      err = [e];
    }
  if (err)
    throw err[0];
  return res;
};

// app/say.js
function say(line) {
  console.log(line);
}
var init_say = __esm(() => {
  say("eval say");
});

// app/data.json
var require_data = __commonJS(function(exports2, module2) {
  module2.exports = { name: "fixture", items: [1, 2, 3] };
});

// app/cjs.js
var require_cjs = __commonJS(function(exports2, module2) {
  var path = require("path");
  console.log("eval cjs");
  exports2.join = function(a, b) {
    return path.posix.join(a, b);
  };
  exports2.tag = "cjs-" + typeof module2;
});

// app/cycle-b.js
var require_cycle_b = __commonJS(function(exports2) {
  console.log("eval cycle-b");
  var a = require_cycle_a();
  exports2.seen = "b saw " + a.early + "," + a.late;
  exports2.check = function() {
    return "b later " + a.late;
  };
});

// app/cycle-a.js
var require_cycle_a = __commonJS(function(exports2) {
  console.log("eval cycle-a start");
  exports2.early = "a-early";
  var b = require_cycle_b();
  exports2.late = "a-late";
  exports2.report = function() {
    return b.seen + " " + b.check();
  };
  console.log("eval cycle-a end");
});

// app/lazy.js
var value = 42, lazy_default = "lazy-default";
var init_lazy = __esm(() => {
  init_say();
  say("eval lazy");
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
init_say();
var exports_math = {};
__export(exports_math, {
  add: () => add,
  mul: () => mul,
  triple: () => triple
});
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
say("math " + Object.keys(exports_math).sort().join(",") + " " + add(2, 3));
say("live " + counter);
bump();
say("live " + counter);
say("reexport " + shout("x") + " " + triple(4));
say("json " + import_data.default.name + " " + import_data.default.items.length);
say("cjs " + import_cjs.default.join("a", "b") + " " + import_cjs.default.tag);
say("cycle " + import_cycle_a.default.report());
Promise.resolve().then(() => __toESM(require_data())).then((lazy) => say("json lazy " + (lazy.default || lazy).name));
Promise.resolve().then(() => (init_lazy(), {})).then((lazy) => say("lazy " + value + " " + lazy_default));
})
