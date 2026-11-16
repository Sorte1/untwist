function f(a) { return a + 1; }
var a = []; a.push(Reflect); var s = a[0].get(a[0], "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
