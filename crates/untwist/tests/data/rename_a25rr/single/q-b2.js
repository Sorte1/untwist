var q = 1;
function zz() { return 2; }
var r = (function(g){ var s = false; JSON.stringify(g, function (k, v) { if (k === "q") s = true; return k === "" ? v : undefined; }); return s; })(this);
console.log(typeof r === "object" ? JSON.stringify(r) : r);
