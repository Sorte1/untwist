var q = 1;
function zz() { return 2; }
var r = Array.prototype.some.call(["q"], Math.hasOwnProperty, this);
console.log(typeof r === "object" ? JSON.stringify(r) : r);
