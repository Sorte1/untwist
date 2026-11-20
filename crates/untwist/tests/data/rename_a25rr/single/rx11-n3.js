function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; (function me(p) { var R = p[n]; if (R) { var [{ set: s }] = [R]; s(g, "f", function () { return 99; }); } })(g);
console.log(f(1), f(2));
