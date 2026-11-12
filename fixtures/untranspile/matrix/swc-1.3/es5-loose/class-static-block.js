"use strict";
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var Config = function Config() {
    "use strict";
};
Config.defaults = {
    mode: "fast"
};
(function() {
    Config.ready = Object.keys(Config.defaults).length > 0;
    Config.loaded = "yes";
})();
(function() {
    var _$local = 3;
    Config.triple = _$local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : _type_of(local));
