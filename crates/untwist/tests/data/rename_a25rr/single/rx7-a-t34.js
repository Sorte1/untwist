function f(a) { return a + 1; }
var { r } = { r: Reflect }; var s = r.get(r, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
