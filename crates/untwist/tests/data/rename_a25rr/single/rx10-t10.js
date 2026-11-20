function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var w = { bind: function () { return g; } }; let a = [1]; a.forEach(function (x, i, arr) { Object.setPrototypeOf(arr, w.bind()); }); var R = a[n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
