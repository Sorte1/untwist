function f(a) { return a + 1; }
for (var R of [Reflect]) { R.get(R, "set") (globalThis, "f", function () { return 99; }); }
console.log(f(1), f(2));
