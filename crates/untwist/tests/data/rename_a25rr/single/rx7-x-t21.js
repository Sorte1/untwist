function f(a) { return a + 1; }
var o = {}; o["__proto__"] = Reflect; var s = Reflect.get(o, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
