function length(a) { return a + 1; }
Array.of.call(function () { return globalThis; });
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
