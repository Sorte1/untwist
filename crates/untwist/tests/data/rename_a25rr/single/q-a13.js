var q = 1;
function zz() { return 2; }
var r = __lookupGetter__.constructor("return Object.keys(this).indexOf(\"q\") >= 0")();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
