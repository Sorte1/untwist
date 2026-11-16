function f(a) { return a + 1; }
var R = Reflect; var t = { r: R }; t.r.get(t.r, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
