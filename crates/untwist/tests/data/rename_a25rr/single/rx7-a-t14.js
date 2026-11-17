function f(a) { return a + 1; }
var s = (0, Reflect).get(Reflect, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
