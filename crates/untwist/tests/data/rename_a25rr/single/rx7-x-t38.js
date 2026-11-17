function f(a) { return a + 1; }
var R = (function (x) { return x; })(Reflect); R.get(R, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
