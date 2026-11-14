var q = 1;
function zz() { return 2; }
var r = Array.prototype.hasOwnProperty.call(typeof window !== "undefined" ? window : globalThis, "q");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
