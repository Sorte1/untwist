function f(a) { return a + 1; }
var g = [globalThis][0]; Symbol.for = function () { return "Reflect"; }; var R = g[Symbol.for("x")]; R.set(g, "f", function () { return 99; });
console.log(f(1), f(2));
