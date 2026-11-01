(() => {
  var __webpack_modules__ = {
    "./app/index.js": (module, exports, __webpack_require__) => {
      "use strict";
      var _say = __webpack_require__("./app/say.js");
      var _math = __webpack_require__("./app/math.js");
      (0, _say.c)("eval index");
      (0, _say.c)("math " + (0, _math.t)(4) + " " + _math.n);
      (0, _say.c)("native " + __webpack_require__.ab.slice(-16));
      Promise.resolve()
        .then(__webpack_require__.bind(__webpack_require__, "./app/lazy.js"))
        .then((lazy) => (0, _say.c)("lazy " + lazy.value + " " + lazy.default));
    },
    "./app/say.js": (module, exports, __webpack_require__) => {
      "use strict";
      __webpack_require__.d(exports, { c: () => say });
      function say(line) {
        console.log(line);
      }
      say("eval say");
    },
    "./app/math.js": (module, exports, __webpack_require__) => {
      "use strict";
      __webpack_require__.d(exports, { n: () => count, t: () => triple });
      var _say = __webpack_require__("./app/say.js");
      (0, _say.c)("eval math");
      let count = 3;
      function triple(value) {
        return value * count;
      }
    },
    "./app/lazy.js": (module, exports, __webpack_require__) => {
      "use strict";
      __webpack_require__.r(exports);
      __webpack_require__.d(exports, { default: () => __WEBPACK_DEFAULT_EXPORT__, value: () => value });
      var _say = __webpack_require__("./app/say.js");
      (0, _say.c)("eval lazy");
      const value = 42;
      const __WEBPACK_DEFAULT_EXPORT__ = "lazy-default";
    },
  };
  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cachedModule = __webpack_module_cache__[moduleId];
    if (cachedModule !== undefined) {
      return cachedModule.exports;
    }
    var module = (__webpack_module_cache__[moduleId] = { exports: {} });
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  (() => {
    __webpack_require__.d = (exports, definition) => {
      for (var key in definition) {
        if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
          Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
        }
      }
    };
  })();
  (() => {
    __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);
  })();
  (() => {
    __webpack_require__.r = (exports) => {
      if (typeof Symbol !== "undefined" && Symbol.toStringTag) {
        Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
      }
      Object.defineProperty(exports, "__esModule", { value: true });
    };
  })();
  if (typeof __webpack_require__ !== "undefined") __webpack_require__.ab = __dirname + "/native_modules/";
  var __webpack_exports__ = __webpack_require__((__webpack_require__.s = "./app/index.js"));
})();
