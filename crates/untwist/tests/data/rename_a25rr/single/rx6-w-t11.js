function f(a) { return a + 1; }
var G = Reflect; var s = G.get(Reflect, "set"); s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
