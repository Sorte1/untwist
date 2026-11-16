function f(a) { return a + 1; }
for (var { set: s } of [Reflect]) s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
