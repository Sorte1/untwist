function f(a) { return a + 1; }
var s = Reflect.get(Object.create(Object.create(Reflect)), "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
