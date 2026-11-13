"use strict";
(() => {
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __commonJS = (cb, mod) => function __require() {
    try {
      return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
    } catch (e) {
      throw mod = 0, e;
    }
  };

  // .src-mini-cjs/cjs.js
  var require_cjs = __commonJS({
    ".src-mini-cjs/cjs.js"(exports) {
      console.log("eval cjs");
      exports.twice = function(n) {
        return n * 2;
      };
    }
  });

  // .src-mini-cjs/lazy.js
  var require_lazy = __commonJS({
    ".src-mini-cjs/lazy.js"(exports) {
      "use strict";
      Object.defineProperty(exports, "__esModule", {
        value: true
      });
      exports.value = void 0;
      console.log("eval lazy");
      var value = exports.value = 42;
    }
  });

  // .src-mini-cjs/index.js
  var _cjs = require_cjs();
  var cache;
  function _interopRequireWildcard(e, t) {
    if (!t && e && e.__esModule) return e;
    var r, c, a = Object.defineProperty, n = { __proto__: null, default: e };
    if (Object(e) !== e) return n;
    if (cache || "function" != typeof WeakMap || (cache = /* @__PURE__ */ new WeakMap()), cache) {
      if (cache.has(e)) return cache.get(e);
      cache.set(e, n);
    }
    for (c in e) "default" !== c && {}.hasOwnProperty.call(e, c) && ((r = a && Object.getOwnPropertyDescriptor(e, c)) && (r.get || r.set) ? a(n, c, r) : n[c] = e[c]);
    return n;
  }
  console.log("eval index " + (0, _cjs.twice)(21));
  Promise.resolve().then(() => _interopRequireWildcard(require_lazy())).then((lazy) => console.log("lazy " + lazy.value));
})();
