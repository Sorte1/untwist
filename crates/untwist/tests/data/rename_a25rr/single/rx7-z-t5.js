function f(a) { return a + 1; }
function g() { let x; try { x = Function("return this")(); } catch (e) { x = globalThis; } return x; }
var G = g(); var R = G.Reflect; R.get(R, "set")(G, "f", function () { return 99; });
console.log(f(1), f(2));
