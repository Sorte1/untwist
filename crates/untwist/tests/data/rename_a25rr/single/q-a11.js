var q = 1;
function zz() { return 2; }
var r = String.prototype.hasOwnProperty.call([globalThis][0], "q");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
