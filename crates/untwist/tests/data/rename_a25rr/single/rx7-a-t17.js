function f(a) { return a + 1; }
var s = (function (R) { return R.get(R, "set"); })(Reflect); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
