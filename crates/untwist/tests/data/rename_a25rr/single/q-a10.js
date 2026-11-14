var q = 1;
function zz() { return 2; }
var r = (function(){ var g = globalThis; return Math.hasOwnProperty.call(g, "q"); })();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
