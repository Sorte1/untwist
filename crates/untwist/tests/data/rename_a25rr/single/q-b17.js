var q = 1;
function zz() { return 2; }
var r = isNaN.constructor("return \"q\" in this")();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
