function f(a) { return a + 1; }
var g = (function () { return this; })(); var s = [g.Reflect][0].get(g.Reflect, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
