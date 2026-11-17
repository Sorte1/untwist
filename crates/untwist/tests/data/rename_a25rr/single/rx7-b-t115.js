function f(a) { return a + 1; }
for (var { assign: a } of [Object]) a(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
