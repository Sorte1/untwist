function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var h; (function me(p) { h = me; if (p === g) { var R = p[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; }); } })({}); h(g);
console.log(f(1), f(2));
