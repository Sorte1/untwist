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
  console.log("eval 1");
  module.exports = { value: 2 };
}, 1, []);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval 0", require(_dependencyMap[0]).value);
}, 0, [1]);
__r(0);
