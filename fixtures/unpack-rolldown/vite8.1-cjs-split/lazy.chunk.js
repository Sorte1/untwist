require("./index.js");
//#region tools/rolldown/app/lazy.js
console.log("eval lazy");
var value = 42;
var lazy_default = "lazy-default";
//#endregion
exports.default = lazy_default;
exports.value = value;
