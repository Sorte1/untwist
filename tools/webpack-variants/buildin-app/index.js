var Big = require("./umd.js");
var loaded = require("./loaded.js");
var harmony = require("./harmony.js");
console.log(String(new Big(7)), typeof Big);
console.log("loaded", loaded.atLoad, loaded.later());
console.log("harmony", harmony.kind, harmony.loadedNow());
