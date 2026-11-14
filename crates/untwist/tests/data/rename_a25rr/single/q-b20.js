var q = 1;
function zz() { return 2; }
var r = structuredClone.call.call(Math.hasOwnProperty, this, "q");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
