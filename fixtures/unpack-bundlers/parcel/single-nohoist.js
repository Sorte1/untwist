// modules are defined as an array
// [ module function, map of requires ]
//
// map of requires is short require name -> numeric require
//
// anything defined in a previous bundle is accessed via the
// orig method which is the require for previous bundles

(function (
  modules,
  entry,
  mainEntry,
  parcelRequireName,
  externals,
  distDir,
  publicUrl,
  devServer
) {
  /* eslint-disable no-undef */
  var globalObject =
    typeof globalThis !== 'undefined'
      ? globalThis
      : typeof self !== 'undefined'
      ? self
      : typeof window !== 'undefined'
      ? window
      : typeof global !== 'undefined'
      ? global
      : {};
  /* eslint-enable no-undef */

  // Save the require from previous bundle to this closure if any
  var previousRequire =
    typeof globalObject[parcelRequireName] === 'function' &&
    globalObject[parcelRequireName];

  var importMap = previousRequire.i || {};
  var cache = previousRequire.cache || {};
  // Do not use `require` to prevent Webpack from trying to bundle this call
  var nodeRequire =
    typeof module !== 'undefined' &&
    typeof module.require === 'function' &&
    module.require.bind(module);

  function newRequire(name, jumped) {
    if (!cache[name]) {
      if (!modules[name]) {
        if (externals[name]) {
          return externals[name];
        }
        // if we cannot find the module within our internal map or
        // cache jump to the current global require ie. the last bundle
        // that was added to the page.
        var currentRequire =
          typeof globalObject[parcelRequireName] === 'function' &&
          globalObject[parcelRequireName];
        if (!jumped && currentRequire) {
          return currentRequire(name, true);
        }

        // If there are other bundles on this page the require from the
        // previous one is saved to 'previousRequire'. Repeat this as
        // many times as there are bundles until the module is found or
        // we exhaust the require chain.
        if (previousRequire) {
          return previousRequire(name, true);
        }

        // Try the node require function if it exists.
        if (nodeRequire && typeof name === 'string') {
          return nodeRequire(name);
        }

        var err = new Error("Cannot find module '" + name + "'");
        err.code = 'MODULE_NOT_FOUND';
        throw err;
      }

      localRequire.resolve = resolve;
      localRequire.cache = {};

      var module = (cache[name] = new newRequire.Module(name));

      modules[name][0].call(
        module.exports,
        localRequire,
        module,
        module.exports,
        globalObject
      );
    }

    return cache[name].exports;

    function localRequire(x) {
      var res = localRequire.resolve(x);
      if (res === false) {
        return {};
      }
      // Synthesize a module to follow re-exports.
      if (Array.isArray(res)) {
        var m = {__esModule: true};
        res.forEach(function (v) {
          var key = v[0];
          var id = v[1];
          var exp = v[2] || v[0];
          var x = newRequire(id);
          if (key === '*') {
            Object.keys(x).forEach(function (key) {
              if (
                key === 'default' ||
                key === '__esModule' ||
                Object.prototype.hasOwnProperty.call(m, key)
              ) {
                return;
              }

              Object.defineProperty(m, key, {
                enumerable: true,
                get: function () {
                  return x[key];
                },
              });
            });
          } else if (exp === '*') {
            Object.defineProperty(m, key, {
              enumerable: true,
              value: x,
            });
          } else {
            Object.defineProperty(m, key, {
              enumerable: true,
              get: function () {
                if (exp === 'default') {
                  return x.__esModule ? x.default : x;
                }
                return x[exp];
              },
            });
          }
        });
        return m;
      }
      return newRequire(res);
    }

    function resolve(x) {
      var id = modules[name][1][x];
      return id != null ? id : x;
    }
  }

  function Module(moduleName) {
    this.id = moduleName;
    this.bundle = newRequire;
    this.require = nodeRequire;
    this.exports = {};
  }

  newRequire.isParcelRequire = true;
  newRequire.Module = Module;
  newRequire.modules = modules;
  newRequire.cache = cache;
  newRequire.parent = previousRequire;
  newRequire.distDir = distDir;
  newRequire.publicUrl = publicUrl;
  newRequire.devServer = devServer;
  newRequire.i = importMap;
  newRequire.register = function (id, exports) {
    modules[id] = [
      function (require, module) {
        module.exports = exports;
      },
      {},
    ];
  };

  // Only insert newRequire.load when it is actually used.
  // The code in this file is linted against ES5, so dynamic import is not allowed.
  // INSERT_LOAD_HERE

  Object.defineProperty(newRequire, 'root', {
    get: function () {
      return globalObject[parcelRequireName];
    },
  });

  globalObject[parcelRequireName] = newRequire;

  for (var i = 0; i < entry.length; i++) {
    newRequire(entry[i]);
  }

  if (mainEntry) {
    // Expose entry point to Node, AMD or browser globals
    // Based on https://github.com/ForbesLindesay/umd/blob/master/template.js
    var mainExports = newRequire(mainEntry);

    // CommonJS
    if (typeof exports === 'object' && typeof module !== 'undefined') {
      module.exports = mainExports;

      // RequireJS
    } else if (typeof define === 'function' && define.amd) {
      define(function () {
        return mainExports;
      });
    }
  }
})({"eX82E":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
var _sayJs = require("./say.js");
var _greetJs = require("./greet.js");
var _greetJsDefault = parcelHelpers.interopDefault(_greetJs);
var _mathJs = require("./math.js");
var _liveJs = require("./live.js");
var _reexportJs = require("./reexport.js");
var _dataJson = require("./data.json");
var _dataJsonDefault = parcelHelpers.interopDefault(_dataJson);
var _cjsJs = require("./cjs.js");
var _cjsJsDefault = parcelHelpers.interopDefault(_cjsJs);
var _cycleAJs = require("./cycle-a.js");
var _cycleAJsDefault = parcelHelpers.interopDefault(_cycleAJs);
(0, _sayJs.say)("eval index");
(0, _sayJs.say)("greet " + (0, _greetJsDefault.default)("ada") + " " + (0, _greetJs.shout)("bob"));
(0, _sayJs.say)("math " + Object.keys(_mathJs).sort().join(",") + " " + _mathJs.add(2, 3));
(0, _sayJs.say)("live " + (0, _liveJs.counter));
(0, _liveJs.bump)();
(0, _sayJs.say)("live " + (0, _liveJs.counter));
(0, _sayJs.say)("reexport " + (0, _reexportJs.loud)("x") + " " + (0, _reexportJs.triple)(4));
(0, _sayJs.say)("json " + (0, _dataJsonDefault.default).name + " " + (0, _dataJsonDefault.default).items.length);
(0, _sayJs.say)("cjs " + (0, _cjsJsDefault.default).join("a", "b") + " " + (0, _cjsJsDefault.default).tag);
(0, _sayJs.say)("cycle " + (0, _cycleAJsDefault.default).report());
require("63258cf13b323bb4").then((lazy)=>(0, _sayJs.say)("json lazy " + (lazy.default || lazy).name));

},{"./say.js":"fs3t3","./greet.js":"laYGN","./math.js":"UNe88","./live.js":"4rA20","./reexport.js":"buNNN","./data.json":"2vnaH","./cjs.js":"6eIEg","./cycle-a.js":"hltbs","63258cf13b323bb4":"5B24G","@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"fs3t3":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "say", ()=>say);
function say(line) {
    console.log(line);
}
say("eval say");

},{"@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"lE9pg":[function(require,module,exports,__globalThis) {
exports.interopDefault = function(a) {
    return a && a.__esModule ? a : {
        default: a
    };
};
exports.defineInteropFlag = function(a) {
    Object.defineProperty(a, '__esModule', {
        value: true
    });
};
exports.exportAll = function(source, dest) {
    Object.keys(source).forEach(function(key) {
        if (key === 'default' || key === '__esModule' || Object.prototype.hasOwnProperty.call(dest, key)) return;
        Object.defineProperty(dest, key, {
            enumerable: true,
            get: function() {
                return source[key];
            }
        });
    });
    return dest;
};
exports.export = function(dest, destName, get) {
    Object.defineProperty(dest, destName, {
        enumerable: true,
        get: get
    });
};

},{}],"laYGN":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "default", ()=>greet);
parcelHelpers.export(exports, "shout", ()=>shout);
var _sayJs = require("./say.js");
(0, _sayJs.say)("eval greet");
function greet(name) {
    return "hello " + name;
}
function shout(name) {
    return name.toUpperCase() + "!";
}

},{"./say.js":"fs3t3","@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"UNe88":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "add", ()=>add);
parcelHelpers.export(exports, "mul", ()=>mul);
parcelHelpers.export(exports, "triple", ()=>triple);
var _sayJs = require("./say.js");
(0, _sayJs.say)("eval math");
const add = (a, b)=>a + b;
const mul = (a, b)=>a * b;
function triple(n) {
    return mul(n, 3);
}

},{"./say.js":"fs3t3","@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"4rA20":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "counter", ()=>counter);
parcelHelpers.export(exports, "bump", ()=>bump);
var _sayJs = require("./say.js");
(0, _sayJs.say)("eval live");
let counter = 0;
function bump() {
    counter += 1;
}

},{"./say.js":"fs3t3","@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"buNNN":[function(require,module,exports,__globalThis) {
var parcelHelpers = require("@parcel/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "loud", ()=>(0, _greetJs.shout));
var _sayJs = require("./say.js");
var _greetJs = require("./greet.js");
var _mathJs = require("./math.js");
parcelHelpers.exportAll(_mathJs, exports);
(0, _sayJs.say)("eval reexport");

},{"./say.js":"fs3t3","./greet.js":"laYGN","./math.js":"UNe88","@parcel/transformer-js/src/esmodule-helpers.js":"lE9pg"}],"2vnaH":[function(require,module,exports,__globalThis) {
module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");

},{}],"6eIEg":[function(require,module,exports,__globalThis) {
const path = require("a6e6e760fa94b166");
console.log("eval cjs");
exports.join = function(a, b) {
    return path.posix.join(a, b);
};
exports.tag = "cjs-object";

},{"a6e6e760fa94b166":"path"}],"hltbs":[function(require,module,exports,__globalThis) {
console.log("eval cycle-a start");
exports.early = "a-early";
const b = require("5020b1ad636ec4a2");
exports.late = "a-late";
exports.report = function() {
    return b.seen + " " + b.check();
};
console.log("eval cycle-a end");

},{"5020b1ad636ec4a2":"gzGfb"}],"gzGfb":[function(require,module,exports,__globalThis) {
console.log("eval cycle-b");
const a = require("26847dc6cc4b6bb2");
exports.seen = "b saw " + a.early + "," + a.late;
exports.check = function() {
    return "b later " + a.late;
};

},{"26847dc6cc4b6bb2":"hltbs"}],"5B24G":[function(require,module,exports,__globalThis) {
module.exports = Promise.resolve(module.bundle.root("2vnaH"));

},{}]},["eX82E"], "eX82E", "parcelRequire6b20", {})

