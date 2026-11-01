this._s = this._s || {};
(function (_) {
  try {
    _._ModuleManager_initialize = function () {};
    _._ModuleManager_initialize("a:0/b:0", []);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    _._ModuleManager_initialize("a:0", []);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    console.log("eval a");
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
