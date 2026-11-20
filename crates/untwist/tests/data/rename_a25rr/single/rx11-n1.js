function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; (function me(p, d) { if (d) { me(g, 0); return; } var R = p[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; }); })({}, 1);
console.log(f(1), f(2));
