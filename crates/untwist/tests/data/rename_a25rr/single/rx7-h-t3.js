function f(a) { return a + 1; }
var R = Object.defineProperty(Reflect, "x", { value: 1 }); R.get(R, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
