var q = 1;
function zz() { return 2; }
var r = (function(){ var g = globalThis; return JSON.stringify(g, ["q"]); })();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
