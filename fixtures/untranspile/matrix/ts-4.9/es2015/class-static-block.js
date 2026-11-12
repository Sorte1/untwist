"use strict";
var _a;
class Config {
}
_a = Config;
Config.defaults = { mode: "fast" };
(() => {
    _a.ready = Object.keys(_a.defaults).length > 0;
    Config.loaded = "yes";
})();
(() => {
    let local = 3;
    _a.triple = local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
