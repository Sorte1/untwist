function f(a) { return a + 1; }
var g = [globalThis][0]; var n = "Reflect", m = "Object"; var R = g[n], O = g[m]; var P = function () {}; P.prototype = R; var s = Reflect.get(new P(), "set"); s(g, "f", function () { return 99; });
console.log(f(1), f(2));
