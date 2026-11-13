"use strict";
function _defineProperty(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else {
        obj[key] = value;
    }
    return obj;
}
class Config {
}
_defineProperty(Config, "defaults", {
    mode: "fast"
});
_defineProperty(Config, "ready", void 0);
var __ = {
    writable: true,
    value: (()=>{
        Config.ready = Object.keys(Config.defaults).length > 0;
        Config.loaded = "yes";
    })()
};
var __1 = {
    writable: true,
    value: (()=>{
        let local1 = 3;
        Config.triple = local1 * 3;
    })()
};
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
