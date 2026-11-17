function f(a) { return a + 1; }
var g = [globalThis][0]; var n = "Reflect", m = "Object"; var R = g[n], O = g[m]; (function ({ defineProperty: s }) { s(g, "f", { value: function () { return 99; } }); })(O);
console.log(f(1), f(2));
