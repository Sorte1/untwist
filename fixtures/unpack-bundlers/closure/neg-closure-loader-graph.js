this._s = this._s || {};
(function (_) {
  try {
    _.begin = function (id) {
      console.log("begin " + id);
    };
    _.end = function () {
      console.log("end");
    };
    _._ModuleManager_initialize = function () {};
    _._ModuleManager_initialize("base/sy1:0", []);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    _.begin("main");
    console.log("eval main");
    _.end();
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
