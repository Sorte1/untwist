var q = 1;
function zz() { return 2; }
var r = Math.__proto__.hasOwnProperty.call(this, "q");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
