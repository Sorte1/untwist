function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; (function () { var X = { prototype: g }; var c = { __proto__: X.prototype }; var R = c[n]; (function ({ set }) { set(g, "f", function () { return 99; }); })(R); })();
console.log(f(1), f(2));
