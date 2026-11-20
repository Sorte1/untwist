function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var w = { bind: function () { return g; } }; (function () { var c = {}; Object.setPrototypeOf(c.valueOf(), w.bind()); var R = c[n]; (function ({ set }) { set(g, "f", function () { return 99; }); })(R); })();
console.log(f(1), f(2));
