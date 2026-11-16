function f(a) { return a + 1; }
var o = { get r() { return Reflect; } }; o.r.get(o.r, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
