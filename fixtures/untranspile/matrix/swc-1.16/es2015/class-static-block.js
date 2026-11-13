"use strict";
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else obj[key] = value;
    return obj;
}
class Config {
}
_define_property(Config, "defaults", {
    mode: "fast"
});
_define_property(Config, "ready", void 0);
(()=>{
    Config.ready = Object.keys(Config.defaults).length > 0;
    Config.loaded = "yes";
})();
(()=>{
    let local1 = 3;
    Config.triple = local1 * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
