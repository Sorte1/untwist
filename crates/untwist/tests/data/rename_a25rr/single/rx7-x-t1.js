function f(a) { return a + 1; }
var o = { k: globalThis.Reflect }; var s = o.k.get(o.k, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
