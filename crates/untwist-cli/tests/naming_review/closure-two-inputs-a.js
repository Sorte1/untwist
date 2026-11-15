"use strict";
this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    _.lib = {};
    var sharedFn = function () { return 1; };
    var sharedCls = class {};
    var priv = function () {};
    _.lib.f = sharedFn; _.lib.c = sharedCls;
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    sharedFn(); new sharedCls();
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    sharedFn();
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
