this._s = this._s || {};
(function (_) {
  var window = this;
  try { _._ModuleManager_initialize = function () {}; _.lib = {}; } catch (e) { _._DumpException(e); }
  try { /*_M:a*/ console.log("eval a"); _.lib.a = 1; } catch (e) { _._DumpException(e); }
  try { /*_M:b*/ (class { static m() { "use strict"; } }); zz = 5; console.log("eval b", _.lib.a, zz, (function () { return typeof this; })()); } catch (e) { _._DumpException(e); }
  try { /*_M:c*/ class K2 { static { console.log("eval c", (function () { return typeof this; })()); } } } catch (e) { _._DumpException(e); }
  try { /*_M:d*/ (function () { "use strict"; console.log("eval d", (function () { return typeof this; })()); })(); } catch (e) { _._DumpException(e); }
}).call(this, this._s);
