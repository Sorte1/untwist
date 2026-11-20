function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var w = { bind: function () { return g; } }; let c = {}; var X = { [Symbol.hasInstance](o) { Object.setPrototypeOf(o, w.bind()); } }; c instanceof X; var R = c[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
