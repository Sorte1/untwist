var a, b;
a || (a = 1, b = { v: 1 });
var r = Array.prototype.hasOwnProperty.call(globalThis, "a");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
