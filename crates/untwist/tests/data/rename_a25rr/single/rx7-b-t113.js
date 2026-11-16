function f(a) { return a + 1; }
var { r: { defineProperty: d } } = { r: Object }; d(globalThis, "f", { value: function () { return 99; } });
console.log(f(1), f(2));
