function length(a) { return a + 1; }
function add(list, x) { Array.prototype.push.call(list, x); } add(window0(), 1); function window0() { return this; }
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
