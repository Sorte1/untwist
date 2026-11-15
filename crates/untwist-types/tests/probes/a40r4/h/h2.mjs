Object.prototype.__defineGetter__("exports", function () { return this; });
exports.parseInt = function () { return null; };
exports.isNaN = function () { return "s"; };
const p = parseInt("12");
const n = isNaN(1);
const m = Math.random();
console.log(typeof p, typeof n, typeof m);
