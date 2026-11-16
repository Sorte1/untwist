function length(a) { return a + 1; }
var p = Reflect.get(Array.prototype, "pop"); p.call(globalThis);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
