"use strict";
var Config = /** @class */ (function () {
    function Config() {
    }
    var _a;
    _a = Config;
    Config.defaults = { mode: "fast" };
    (function () {
        _a.ready = Object.keys(_a.defaults).length > 0;
        _a.loaded = "yes";
    })();
    (function () {
        var local = 3;
        _a.triple = local * 3;
    })();
    return Config;
}());
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
