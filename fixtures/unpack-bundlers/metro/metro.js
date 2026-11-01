var __BUNDLE_START_TIME__=globalThis.nativePerformanceNow?nativePerformanceNow():Date.now(),__DEV__=false,process=globalThis.process||{},__METRO_GLOBAL_PREFIX__='';process.env=process.env||{};process.env.NODE_ENV=process.env.NODE_ENV||"production";
(function (global) {
  "use strict";

  global.__r = metroRequire;
  global[`${__METRO_GLOBAL_PREFIX__}__d`] = define;
  global.__c = clear;
  global.__registerSegment = registerSegment;
  var modules = clear();
  const EMPTY = {};
  const CYCLE_DETECTED = {};
  const {
    hasOwnProperty
  } = {};
  function clear() {
    modules = new Map();
    return modules;
  }
  function define(factory, moduleId, dependencyMap) {
    if (modules.has(moduleId)) {
      return;
    }
    const mod = {
      dependencyMap,
      factory,
      hasError: false,
      importedAll: EMPTY,
      importedDefault: EMPTY,
      isInitialized: false,
      publicModule: {
        exports: {}
      }
    };
    modules.set(moduleId, mod);
  }
  function metroRequire(moduleId, maybeNameForDev) {
    if (moduleId === null) {
      throw new Error("Cannot find module");
    }
    const moduleIdReallyIsNumber = moduleId;
    const module = modules.get(moduleIdReallyIsNumber);
    return module && module.isInitialized ? module.publicModule.exports : guardedLoadModule(moduleIdReallyIsNumber, module);
  }
  function metroImportDefault(moduleId) {
    const moduleIdReallyIsNumber = moduleId;
    const maybeInitializedModule = modules.get(moduleIdReallyIsNumber);
    if (maybeInitializedModule && maybeInitializedModule.importedDefault !== EMPTY) {
      return maybeInitializedModule.importedDefault;
    }
    const exports = metroRequire(moduleIdReallyIsNumber);
    const importedDefault = exports && exports.__esModule ? exports.default : exports;
    const initializedModule = modules.get(moduleIdReallyIsNumber);
    return initializedModule.importedDefault = importedDefault;
  }
  metroRequire.importDefault = metroImportDefault;
  function metroImportAll(moduleId) {
    const moduleIdReallyIsNumber = moduleId;
    const maybeInitializedModule = modules.get(moduleIdReallyIsNumber);
    if (maybeInitializedModule && maybeInitializedModule.importedAll !== EMPTY) {
      return maybeInitializedModule.importedAll;
    }
    const exports = metroRequire(moduleIdReallyIsNumber);
    let importedAll;
    if (exports && exports.__esModule) {
      importedAll = exports;
    } else {
      importedAll = {};
      if (exports) {
        for (const key in exports) {
          if (hasOwnProperty.call(exports, key)) {
            importedAll[key] = exports[key];
          }
        }
      }
      importedAll.default = exports;
    }
    const initializedModule = modules.get(moduleIdReallyIsNumber);
    return initializedModule.importedAll = importedAll;
  }
  metroRequire.importAll = metroImportAll;
  metroRequire.context = function fallbackRequireContext() {
    throw new Error("The experimental Metro feature `require.context` is not enabled in your project.");
  };
  metroRequire.resolveWeak = function fallbackRequireResolveWeak() {
    throw new Error("require.resolveWeak cannot be called dynamically.");
  };
  let inGuard = false;
  function guardedLoadModule(moduleId, module) {
    if (!inGuard && global.ErrorUtils) {
      inGuard = true;
      let returnValue;
      try {
        returnValue = loadModuleImplementation(moduleId, module);
      } catch (e) {
        global.ErrorUtils.reportFatalError(e);
      }
      inGuard = false;
      return returnValue;
    } else {
      return loadModuleImplementation(moduleId, module);
    }
  }
  const ID_MASK_SHIFT = 16;
  const LOCAL_ID_MASK = 65535;
  function unpackModuleId(moduleId) {
    const segmentId = moduleId >>> ID_MASK_SHIFT;
    const localId = moduleId & LOCAL_ID_MASK;
    return {
      segmentId,
      localId
    };
  }
  metroRequire.unpackModuleId = unpackModuleId;
  function packModuleId(value) {
    return (value.segmentId << ID_MASK_SHIFT) + value.localId;
  }
  metroRequire.packModuleId = packModuleId;
  const moduleDefinersBySegmentID = [];
  const definingSegmentByModuleID = new Map();
  function registerSegment(segmentId, moduleDefiner, moduleIds) {
    moduleDefinersBySegmentID[segmentId] = moduleDefiner;
    if (moduleIds) {
      moduleIds.forEach(moduleId => {
        if (!modules.has(moduleId) && !definingSegmentByModuleID.has(moduleId)) {
          definingSegmentByModuleID.set(moduleId, segmentId);
        }
      });
    }
  }
  function loadModuleImplementation(moduleId, module) {
    if (!module && moduleDefinersBySegmentID.length > 0) {
      const segmentId = definingSegmentByModuleID.get(moduleId) ?? 0;
      const definer = moduleDefinersBySegmentID[segmentId];
      if (definer != null) {
        definer(moduleId);
        module = modules.get(moduleId);
        definingSegmentByModuleID.delete(moduleId);
      }
    }
    const nativeRequire = global.nativeRequire;
    if (!module && nativeRequire) {
      const {
        segmentId,
        localId
      } = unpackModuleId(moduleId);
      nativeRequire(localId, segmentId);
      module = modules.get(moduleId);
    }
    if (!module) {
      throw unknownModuleError(moduleId);
    }
    if (module.hasError) {
      throw module.error;
    }
    module.isInitialized = true;
    const {
      factory,
      dependencyMap
    } = module;
    try {
      const moduleObject = module.publicModule;
      moduleObject.id = moduleId;
      factory(global, metroRequire, metroImportDefault, metroImportAll, moduleObject, moduleObject.exports, dependencyMap);
      {
        module.factory = undefined;
        module.dependencyMap = undefined;
      }
      return moduleObject.exports;
    } catch (e) {
      module.hasError = true;
      module.error = e;
      module.isInitialized = false;
      module.publicModule.exports = undefined;
      throw e;
    } finally {}
  }
  function unknownModuleError(id) {
    let message = 'Requiring unknown module "' + id + '".';
    return Error(message);
  }
})(typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof window !== 'undefined' ? window : this);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  var say = require(_dependencyMap[0]).say;
  var greet = _$$_IMPORT_DEFAULT(_dependencyMap[1]);
  var shout = require(_dependencyMap[1]).shout;
  var math = _$$_IMPORT_ALL(_dependencyMap[2]);
  var _liveJs = require(_dependencyMap[3]),
    counter = _liveJs.counter,
    bump = _liveJs.bump;
  var _reexportJs = require(_dependencyMap[4]),
    loud = _reexportJs.loud,
    triple = _reexportJs.triple;
  var data = _$$_IMPORT_DEFAULT(_dependencyMap[5]);
  var cjs = _$$_IMPORT_DEFAULT(_dependencyMap[6]);
  var cycle = _$$_IMPORT_DEFAULT(_dependencyMap[7]);
  say("eval index");
  say("greet " + greet("ada") + " " + shout("bob"));
  say("math " + Object.keys(math).sort().join(",") + " " + math.add(2, 3));
  say("live " + counter);
  bump();
  say("live " + counter);
  say("reexport " + loud("x") + " " + triple(4));
  say("json " + data.name + " " + data.items.length);
  say("cjs " + cjs.join("a", "b") + " " + cjs.tag);
  say("cycle " + cycle.report());
  require(_dependencyMap[9])(_dependencyMap[8], _dependencyMap.paths).then(lazy => say("json lazy " + (lazy.default || lazy).name));
  require(_dependencyMap[9])(_dependencyMap[10], _dependencyMap.paths).then(lazy => say("lazy " + lazy.value + " " + lazy.default));
},0,[1,2,3,4,5,6,7,9,6,11,12]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  function say(line) {
    console.log(line);
  }
  say("eval say");
  exports.say = say;
},1);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  var say = require(_dependencyMap[0]).say;
  say("eval greet");
  function greet(name) {
    return "hello " + name;
  }
  function shout(name) {
    return name.toUpperCase() + "!";
  }
  exports.shout = shout;
  exports.default = greet;
},2,[1]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  var say = require(_dependencyMap[0]).say;
  say("eval math");
  const add = (a, b) => a + b;
  const mul = (a, b) => a * b;
  function triple(n) {
    return mul(n, 3);
  }
  exports.add = add;
  exports.mul = mul;
  exports.triple = triple;
},3,[1]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  var say = require(_dependencyMap[0]).say;
  say("eval live");
  let counter = 0;
  function bump() {
    counter += 1;
  }
  exports.counter = counter;
  exports.bump = bump;
},4,[1]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  var say = require(_dependencyMap[0]).say;
  var _shout = require(_dependencyMap[1]).shout;
  var _mathJs = require(_dependencyMap[2]);
  for (var _key in _mathJs) {
    if (_key === "default") continue;
    exports[_key] = _mathJs[_key];
  }
  say("eval reexport");
  exports.loud = _shout;
},5,[1,2,3]);
__d(function(global, require, _importDefaultUnused, _importAllUnused, module, exports, _dependencyMapUnused) {
  module.exports = { "name": "fixture", "items": [1, 2, 3] }
;
},6);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  const path = require(_dependencyMap[0]);
  console.log("eval cjs");
  exports.join = function (a, b) {
    return path.posix.join(a, b);
  };
  exports.tag = "cjs-" + typeof module;
},7,[8]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  module.exports = process.getBuiltinModule("path");
},8);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval cycle-a start");
  exports.early = "a-early";
  const b = require(_dependencyMap[0]);
  exports.late = "a-late";
  exports.report = function () {
    return b.seen + " " + b.check();
  };
  console.log("eval cycle-a end");
},9,[10]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval cycle-b");
  const a = require(_dependencyMap[0]);
  exports.seen = "b saw " + a.early + "," + a.late;
  exports.check = function () {
    return "b later " + a.late;
  };
},10,[9]);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  function maybeLoadBundle(moduleID, paths) {
    const loadBundle = global[`${__METRO_GLOBAL_PREFIX__}__loadBundleAsync`];
    if (loadBundle != null) {
      const stringModuleID = String(moduleID);
      if (paths != null) {
        const bundlePath = paths[stringModuleID];
        if (bundlePath != null) {
          return loadBundle(bundlePath);
        }
      }
    }
    return undefined;
  }
  function asyncRequireImpl(moduleID, paths) {
    const maybeLoadBundlePromise = maybeLoadBundle(moduleID, paths);
    const importAll = () => require.importAll(moduleID);
    if (maybeLoadBundlePromise != null) {
      return maybeLoadBundlePromise.then(importAll);
    }
    return importAll();
  }
  async function asyncRequire(moduleID, paths, moduleName) {
    return asyncRequireImpl(moduleID, paths);
  }
  asyncRequire.unstable_importMaybeSync = function unstable_importMaybeSync(moduleID, paths) {
    return asyncRequireImpl(moduleID, paths);
  };
  asyncRequire.prefetch = function (moduleID, paths, moduleName) {
    maybeLoadBundle(moduleID, paths)?.then(() => {}, () => {});
  };
  module.exports = asyncRequire;
},11);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  "use strict";

  Object.defineProperty(exports, '__esModule', {
    value: true
  });
  var say = require(_dependencyMap[0]).say;
  say("eval lazy");
  const value = 42;
  var _default = "lazy-default";
  exports.value = value;
  exports.default = _default;
},12,[1]);
__r(0);