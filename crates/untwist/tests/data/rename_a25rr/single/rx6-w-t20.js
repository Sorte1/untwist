function f(a) { return a + 1; }
var h = { r: Reflect }; var s = Reflect.get(h.r, "set"); s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
