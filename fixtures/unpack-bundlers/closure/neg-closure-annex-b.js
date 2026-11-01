this._s = this._s || {};
(function (_) {
  try {
    _._ModuleManager_initialize = function () {};
    _._DumpException = function (e) {
      console.log("caught", e.message);
    };
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    function shared() {
      return "shared";
    }
    console.log("eval a");
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    console.log("eval b", shared());
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
