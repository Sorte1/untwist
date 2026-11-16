function f(a) { return a + 1; }
(function ({ Reflect: { set: s } }) { s (globalThis, "f", function () { return 99; }); })(globalThis);
console.log(f(1), f(2));
