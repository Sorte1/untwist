"use strict";
function _classCallCheck(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
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
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var Config = function Config() {
    "use strict";
    _classCallCheck(this, Config);
};
_defineProperty(Config, "defaults", {
    mode: "fast"
});
_defineProperty(Config, "ready", void 0);
var __ = {
    writable: true,
    value: function() {
        Config.ready = Object.keys(Config.defaults).length > 0;
        Config.loaded = "yes";
    }()
};
var __1 = {
    writable: true,
    value: function() {
        var _$local = 3;
        Config.triple = _$local * 3;
    }()
};
console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : _typeof(local));
