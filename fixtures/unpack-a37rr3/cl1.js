this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    _.lib = {};
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    console.log("eval a");
    _.lib.a = 1;
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    class K {}
    zz = 5;
    console.log("eval b", _.lib.a, zz, typeof K, (function () { return typeof this; })());
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
