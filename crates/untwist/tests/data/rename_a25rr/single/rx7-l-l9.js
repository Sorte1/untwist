function length(a) { return a + 1; }
[].splice.call(globalThis, 0);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
