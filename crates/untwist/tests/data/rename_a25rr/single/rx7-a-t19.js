function f(a) { return a + 1; }
function g() { return this.get(Reflect, "set"); } var s = g.call(Reflect); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
