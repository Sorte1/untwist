var q = 1;
function zz() { return 2; }
var r = Math.hasOwnProperty.bind(this)("q");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
