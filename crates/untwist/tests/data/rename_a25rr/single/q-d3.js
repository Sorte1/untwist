var a, b;
a || (a = 1, b = { v: 1 });
var r = (function (g) { return Array.prototype.hasOwnProperty.call(g, "a"); })(typeof window !== "undefined" ? window : globalThis);
console.log(typeof r === "object" ? JSON.stringify(r) : r);
