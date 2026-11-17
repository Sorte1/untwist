function length(a) { return a + 1; }
Reflect.apply(Array.prototype.push, globalThis, []);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
