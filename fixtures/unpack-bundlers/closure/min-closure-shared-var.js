this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    var log = function (line) {
      console.log(line);
    };
    var count = 0;
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:base*/
    var base = function (name) {
      count++;
      return "base:" + name + ":" + count;
    };
    var own = "own";
    var pending;
    log("eval base " + own + " " + typeof later);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:feature*/
    pending = base("feature");
    var later = function () {
      return pending + "/" + count;
    };
    log("eval feature " + pending);
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:main*/
    count += 10;
    log("eval main " + later() + " " + base.call(null, "main") + " " + typeof window);
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
