function f(a) { return a + 1; }
try { throw {}; } catch ({ constructor: { defineProperty: d } }) { d(globalThis, "f", { value: function () { return 99; } }); }
console.log(f(1), f(2));
