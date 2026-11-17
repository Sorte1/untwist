function f(a) { return a + 1; }
[Reflect].forEach(function (R) { R.get(R, "set") (globalThis, "f", function () { return 99; }); });
console.log(f(1), f(2));
