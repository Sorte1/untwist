function f(a) { return a + 1; }
var g = [globalThis][0]; var n = "Reflect", m = "Object"; var R = g[n], O = g[m]; function w({ set: s } = R) { s(g, "f", function () { return 99; }); } w();
console.log(f(1), f(2));
