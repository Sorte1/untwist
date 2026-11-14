var a, b;
a || (a = 1, b = { v: 1 });
var r = Math.constructor.keys(this).indexOf("a") >= 0;
console.log(typeof r === "object" ? JSON.stringify(r) : r);
