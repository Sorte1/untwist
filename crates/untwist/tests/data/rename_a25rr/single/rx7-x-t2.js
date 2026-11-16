function f(a) { return a + 1; }
var R = this.Reflect; var s = (0, R).get(R, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
