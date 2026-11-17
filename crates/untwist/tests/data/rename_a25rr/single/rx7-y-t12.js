function length(a) { return a + 1; }
var [{ push: p }] = [[]]; globalThis.q = p; globalThis.q();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
