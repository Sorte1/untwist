function f(a) { return a + 1; }
var { r: { set: s } } = { r: Reflect }; s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
