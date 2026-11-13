const require_index = require('./index.js');

//#region tools/rolldown/app/lazy.js
console.log("eval lazy");
const value = 42;
var lazy_default = "lazy-default";

//#endregion
exports.default = lazy_default;
exports.value = value;