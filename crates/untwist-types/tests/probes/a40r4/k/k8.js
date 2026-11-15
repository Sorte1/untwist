Object.prototype.__defineGetter__("exports", function () { return this; });
const S = exports.String;
S.prototype.toUpperCase = function () { return 5; };
exports.Math.floor = () => "s";
const u = "abc".toUpperCase(); const fl = Math.floor(1.5);
console.log(typeof u, typeof fl);
export {};
