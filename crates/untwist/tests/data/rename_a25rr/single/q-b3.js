var q = 1;
function zz() { return 2; }
var r = JSON.stringify(this, ["q", "zz"]);
console.log(typeof r === "object" ? JSON.stringify(r) : r);
