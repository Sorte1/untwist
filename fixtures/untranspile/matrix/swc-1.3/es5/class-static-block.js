"use strict";
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _define_property(obj, key, value) {
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
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var Config = function Config() {
    "use strict";
    _class_call_check(this, Config);
};
_define_property(Config, "defaults", {
    mode: "fast"
});
_define_property(Config, "ready", void 0);
(function() {
    Config.ready = Object.keys(Config.defaults).length > 0;
    Config.loaded = "yes";
})();
(function() {
    var _$local = 3;
    Config.triple = _$local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : _type_of(local));
