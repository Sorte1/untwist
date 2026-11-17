function length(a) { return a + 1; }
function g() { let x; try { x = Function("throw 0")(); } catch (e) { x = globalThis; } [].push.call(x); return x; } g();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
