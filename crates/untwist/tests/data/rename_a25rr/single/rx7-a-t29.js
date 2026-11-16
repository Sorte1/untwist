function f(a) { return a + 1; }
[Reflect].map(function (R) { return R.get(R, "set"); })[0] (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
