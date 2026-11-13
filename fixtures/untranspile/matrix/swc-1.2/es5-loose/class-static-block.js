"use strict";
var id = 0;
function _classPrivateFieldLooseKey(name) {
    return "__private_" + id++ + "_" + name;
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var __ = /*#__PURE__*/ _classPrivateFieldLooseKey("__"), __1 = /*#__PURE__*/ _classPrivateFieldLooseKey("__1");
var Config = function Config() {
    "use strict";
};
Config.defaults = {
    mode: "fast"
};
Config.ready = void 0;
Object.defineProperty(Config, __, {
    writable: true,
    value: function() {
        Config.ready = Object.keys(Config.defaults).length > 0;
        Config.loaded = "yes";
    }()
});
Object.defineProperty(Config, __1, {
    writable: true,
    value: function() {
        var _$local = 3;
        Config.triple = _$local * 3;
    }()
});
console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : _typeof(local));
