this._s = this._s || {};
(function (_) {
  try {
    _._ModuleManager_initialize = function () {};
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    for (var shared = "a"; !shared; ) {}
    console.log("eval a");
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    console.log("eval b", shared);
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
