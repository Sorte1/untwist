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
    _.lib.a = 1; _.lib.d = __filename;
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    console.log("eval b", _.lib.a, __filename === _.lib.d);
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
