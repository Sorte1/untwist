function f(a) { return a + 1; }
var k = "Reflect"; (function ({ [k]: R }) { R.get(R, "set") (globalThis, "f", function () { return 99; }); })(globalThis);
console.log(f(1), f(2));
