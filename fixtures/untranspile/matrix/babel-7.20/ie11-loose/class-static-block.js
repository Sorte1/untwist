"use strict";

var Config = function Config() {};
Config.defaults = {
  mode: "fast"
};
Config.ready = void 0;
(function () {
  Config.ready = Object.keys(Config.defaults).length > 0;
  Config.loaded = "yes";
})();
(function () {
  var local = 3;
  Config.triple = local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
