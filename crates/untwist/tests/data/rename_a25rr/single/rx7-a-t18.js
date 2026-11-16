function f(a) { return a + 1; }
var a = Reflect, b = a, c = b, d = c, e = d, g = e, h = g, i = h; var s = i.get(i, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
