this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    _.lib = { order: [] };
    _.note = function (name) {
      _.lib.order.push(name);
      console.log("eval", name, _.lib.order.join(","));
    };
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:base*/
    _.note("base");
    _.lib.base = function () {
      return "base";
    };
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:feature*/
    _.note("feature");
    _.lib.feature = _.lib.base() + "+feature";
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:main*/
    _.note("main");
    console.log(_.lib.feature, window === this);
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
