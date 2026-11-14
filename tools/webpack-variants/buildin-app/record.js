var hooks = require("./hooks.js");
var free = require("./free-module.js");
console.log("hooks", hooks.require, hooks.paths, hooks.polyfilled, hooks.deprecate, hooks.loadedLater());
console.log("free", free.version, require("./children.js"));
