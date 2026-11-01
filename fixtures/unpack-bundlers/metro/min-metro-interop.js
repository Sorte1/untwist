var registry = new Map();
var NONE = {};
var own = Object.prototype.hasOwnProperty;
function __d(factory, moduleId, dependencyMap) {
  registry.set(moduleId, { factory: factory, dependencyMap: dependencyMap, module: null, interopDefault: NONE, interopAll: NONE });
}
function load(moduleId) {
  var record = registry.get(moduleId);
  if (record.module) return record.module.exports;
  var module = (record.module = { exports: {} });
  record.factory(globalThis, load, interopDefault, interopAll, module, module.exports, record.dependencyMap);
  return module.exports;
}
function interopDefault(moduleId) {
  var record = registry.get(moduleId);
  if (record.interopDefault !== NONE) return record.interopDefault;
  var value = load(moduleId);
  return (registry.get(moduleId).interopDefault = value && value.__esModule ? value.default : value);
}
function interopAll(moduleId) {
  var record = registry.get(moduleId);
  if (record.interopAll !== NONE) return record.interopAll;
  var value = load(moduleId);
  var all;
  if (value && value.__esModule) {
    all = value;
  } else {
    all = {};
    if (value) {
      for (var key in value) {
        if (own.call(value, key)) all[key] = value[key];
      }
    }
    all.default = value;
  }
  return (registry.get(moduleId).interopAll = all);
}
load.importDefault = interopDefault;
load.importAll = interopAll;
var __r = load;
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval esm");
  Object.defineProperty(exports, "__esModule", { value: true });
  exports.default = "esm-default";
  exports.named = "esm-named";
}, 1, []);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval cjs");
  module.exports = { a: 1, b: 2 };
}, 2, []);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval fn");
  module.exports = function fn() {
    return "called";
  };
  module.exports.extra = "x";
  module.exports.__esModule = false;
}, 3, []);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  var esmDefault = _$$_IMPORT_DEFAULT(_dependencyMap[0]);
  var esmAll = _$$_IMPORT_ALL(_dependencyMap[0]);
  console.log("esm", esmDefault, esmAll.named, esmAll === require(_dependencyMap[0]));
  var cjsDefault = _$$_IMPORT_DEFAULT(_dependencyMap[1]);
  var cjsAll = _$$_IMPORT_ALL(_dependencyMap[1]);
  console.log("cjs", cjsDefault.a, Object.keys(cjsAll).sort().join(","), cjsAll.default === cjsDefault);
  require(_dependencyMap[1]).c = 3;
  console.log("cached", _$$_IMPORT_ALL(_dependencyMap[1]) === cjsAll, "c" in cjsAll, _$$_IMPORT_DEFAULT(_dependencyMap[1]).c);
  var fnAll = require.importAll(_dependencyMap[2]);
  var fnDefault = require.importDefault(_dependencyMap[2]);
  console.log("fn", fnDefault(), Object.keys(fnAll).sort().join(","), fnAll.default === fnDefault);
  var ids = [_dependencyMap[0], _dependencyMap[1]];
  console.log("by id", ids.map(function (id) { return typeof _$$_IMPORT_DEFAULT(id); }).join(","));
}, 0, [1, 2, 3]);
__r(0);
