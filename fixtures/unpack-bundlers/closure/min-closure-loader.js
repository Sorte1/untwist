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
    _._ModuleManager_initialize("base/sy1:0/main:0,1", []);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    _.begin("sy1");
    _.value = 1;
    console.log("eval sy1");
    _.end();
  } catch (e) {
    _._DumpException(e);
  }
  try {
    _.begin("main");
    console.log("eval main", _.value);
    _.end();
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
