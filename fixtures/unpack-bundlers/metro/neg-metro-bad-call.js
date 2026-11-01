var modules = Object.create(null);
function __d(factory, moduleId, dependencyMap) {
  if (moduleId === undefined) return;
  modules[moduleId] = { factory: factory, dependencyMap: dependencyMap, module: null };
}
function __r(moduleId) {
  var record = modules[moduleId];
  if (record.module) return record.module.exports;
  var module = (record.module = { exports: {} });
  record.factory(globalThis, __r, null, null, module, module.exports, record.dependencyMap);
  return module.exports;
}
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("eval 1");
}, 1, []);
__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  console.log("never");
});
__r(1);
