(() => {
  var __webpack_modules__ = {
    "./src/index.js": (module, exports, __webpack_require__) => {
      "use strict";
      __webpack_require__.r(exports);
      var r = __webpack_require__("./src/add.wasm");
      const c = r.add;
      console.log(c(1, 2), (0, r.add)(3, 4));
    },
    "./src/add.wasm": (module, exports, __webpack_require__) => {
      "use strict";
      var wasmExports = __webpack_require__.w[module.id];
      __webpack_require__.r(exports);
      for (var name in wasmExports) if (name) exports[name] = wasmExports[name];
      var m0 = __webpack_require__("./src/env.js");
      wasmExports[""]();
    },
    "./src/env.js": (module, exports, __webpack_require__) => {
      "use strict";
      __webpack_require__.d(exports, { a: () => o });
      function o(e) {
        console.log(e);
      }
    },
  };
  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cachedModule = __webpack_module_cache__[moduleId];
    if (cachedModule !== undefined) {
      return cachedModule.exports;
    }
    var module = (__webpack_module_cache__[moduleId] = { id: moduleId, exports: {} });
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  __webpack_require__.c = __webpack_module_cache__;
  __webpack_require__.d = (exports, definition) => {
    for (var key in definition) {
      if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
        Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
      }
    }
  };
  __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);
  __webpack_require__.r = (exports) => {
    if (typeof Symbol !== "undefined" && Symbol.toStringTag) {
      Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
    }
    Object.defineProperty(exports, "__esModule", { value: true });
  };
  __webpack_require__.p = "/static/";
  (() => {
    var installedWasmModules = {};
    var wasmImportedFuncCache0;
    var wasmImportObjects = {
      "./src/add.wasm": function () {
        return {
          "./env.js": {
            log: function (p0i32) {
              if (wasmImportedFuncCache0 === undefined) wasmImportedFuncCache0 = __webpack_require__.c["./src/env.js"].exports;
              return wasmImportedFuncCache0["a"](p0i32);
            },
          },
        };
      },
    };
    var wasmModuleMap = { main: ["./src/add.wasm"] };
    __webpack_require__.w = {};
    __webpack_require__.wasm = function (chunkId) {
      var promises = [];
      (wasmModuleMap[chunkId] || []).forEach(function (wasmModuleId) {
        var installed = installedWasmModules[wasmModuleId];
        if (installed) {
          promises.push(installed);
          return;
        }
        var importObject = wasmImportObjects[wasmModuleId]();
        var req = fetch(__webpack_require__.p + "" + { main: { "./src/add.wasm": "9c8b7a6f5e4d3c2b" } }[chunkId][wasmModuleId] + ".module.wasm");
        promises.push(
          (installedWasmModules[wasmModuleId] = req
            .then((x) => x.arrayBuffer())
            .then((bytes) => WebAssembly.instantiate(bytes, importObject))
            .then((res) => (__webpack_require__.w[wasmModuleId] = (res.instance || res).exports))),
        );
      });
      return Promise.all(promises);
    };
  })();
  __webpack_require__.wasm("main").then(() => __webpack_require__("./src/index.js"));
})();
