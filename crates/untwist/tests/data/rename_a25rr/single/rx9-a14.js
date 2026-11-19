function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var c = {}; Reflect.set(c, "__proto__", g); var R = c[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
