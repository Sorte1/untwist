function length(a) { return a + 1; }
var p = [].push; p.call(globalThis);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
