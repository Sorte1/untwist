function f(a) { return a + 1; }
var g = [globalThis][0]; var i = 0; { function i() {} } i.toString = function () { return "Reflect"; }; var [{ set: s }] = [g[i]]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
