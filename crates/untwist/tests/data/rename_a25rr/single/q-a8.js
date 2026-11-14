var q = 1;
function zz() { return 2; }
var r = propertyIsEnumerable.call(this, "zz");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
