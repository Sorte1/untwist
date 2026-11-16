function length(a) { return a + 1; }
var o = { p: [].push }; o.p.call(globalThis);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
