function f(a) { return a + 1; }
var g = [globalThis][0]; var s; Object.getOwnPropertyNames(g).forEach(function (k) { if (k === "Reflect") s = g[k].set; }); s(g, "f", function () { return 99; });
console.log(f(1), f(2));
