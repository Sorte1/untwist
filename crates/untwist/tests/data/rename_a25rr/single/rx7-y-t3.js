function length(a) { return a + 1; }
globalThis.p = [].push; globalThis.p();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
