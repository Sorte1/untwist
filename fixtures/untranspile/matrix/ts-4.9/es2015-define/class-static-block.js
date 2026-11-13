"use strict";
var _a;
class Config {
}
_a = Config;
Object.defineProperty(Config, "defaults", {
    enumerable: true,
    configurable: true,
    writable: true,
    value: { mode: "fast" }
});
(() => {
    _a.ready = Object.keys(_a.defaults).length > 0;
    Config.loaded = "yes";
})();
(() => {
    let local = 3;
    _a.triple = local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
