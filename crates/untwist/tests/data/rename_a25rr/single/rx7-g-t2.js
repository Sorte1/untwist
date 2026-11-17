function length(a) { return a + 1; }
var g = this; [].push.call(g);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
