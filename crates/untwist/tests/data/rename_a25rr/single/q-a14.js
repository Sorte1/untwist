var q = 1;
function zz() { return 2; }
var r = Array.constructor("return \"q\" in this")();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
