function length(a) { return a + 1; }
Reflect.get([], "push").call(globalThis);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
