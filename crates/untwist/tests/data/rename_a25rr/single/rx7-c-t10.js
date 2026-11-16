function f(a) { return a + 1; }
var s = Reflect.get({ get x() { return this; } }, "x", Reflect); s.get(s, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
