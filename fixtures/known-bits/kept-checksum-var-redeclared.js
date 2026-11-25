function f(n) { return 1; }
var f = function () { return 2; };
const seed = (typeof Date !== "undefined" ? Date.now() & 0x7fffc : 4) & 0x7fffc | (typeof f === "function" && f(0) === 1 ? 1 : 0);
console.log((Math.imul(seed ^ 8, seed ^ 8) & 7) === 1 ? "match" : "mismatch");
