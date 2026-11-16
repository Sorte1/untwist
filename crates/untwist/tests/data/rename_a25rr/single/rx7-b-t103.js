function f(a) { return a + 1; }
(function ({ set: s }) { s (globalThis, "f", function () { return 99; }); })(Reflect);
console.log(f(1), f(2));
