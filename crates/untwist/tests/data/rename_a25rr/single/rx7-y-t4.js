function length(a) { return a + 1; }
function h() { return this; } Array.prototype.pop.call(h());
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
