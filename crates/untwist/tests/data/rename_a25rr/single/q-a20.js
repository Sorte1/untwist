var q = 1;
function zz() { return 2; }
var r = Array.prototype.includes.call(Math.constructor.getOwnPropertyNames(this), "zz");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
