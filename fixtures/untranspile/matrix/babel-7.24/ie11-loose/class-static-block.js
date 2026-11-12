"use strict";

var _Config;
var Config = function Config() {};
_Config = Config;
Config.defaults = {
  mode: "fast"
};
Config.ready = void 0;
(function () {
  _Config.ready = Object.keys(_Config.defaults).length > 0;
  _Config.loaded = "yes";
})();
(function () {
  var local = 3;
  _Config.triple = local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
