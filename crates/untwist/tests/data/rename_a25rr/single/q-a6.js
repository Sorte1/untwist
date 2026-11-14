var q = 1;
function zz() { return 2; }
var r = (function(){ var s = false; JSON.stringify(this, function (k, v) { if (k === "q") s = true; return k === "" ? v : undefined; }); return s; })();
console.log(typeof r === "object" ? JSON.stringify(r) : r);
