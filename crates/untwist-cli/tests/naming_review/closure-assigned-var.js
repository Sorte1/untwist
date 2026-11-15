"use strict";
this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    _.lib = {};
    var sharedFn, sharedOr, sharedNu; sharedFn = function () { return 1; }; sharedOr ||= function () {}; sharedNu ??= () => 1;
    var sharedCls = class {};
    var priv = function () {};
    console.log("base", JSON.stringify([sharedFn.name, sharedOr.name, sharedNu.name, sharedCls.name, priv.name]));
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    console.log("eval a", JSON.stringify([sharedFn.name, sharedCls.name]), sharedFn());
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    console.log("eval b", JSON.stringify([sharedFn.name, sharedOr.name, sharedNu.name]));
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
