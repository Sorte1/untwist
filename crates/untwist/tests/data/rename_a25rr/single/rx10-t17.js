function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var w = { bind: function () { return g; } }; let F = function me() { Object.setPrototypeOf(me, w.bind()); }; F.call(); var R = F[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
