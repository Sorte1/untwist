function length(a) { return a + 1; }
var p = Array.prototype.push; (function () { p.call(this); })();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
