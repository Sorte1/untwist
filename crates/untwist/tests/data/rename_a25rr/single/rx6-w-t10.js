function f(a) { return a + 1; }
var R = [Reflect][0]; var s = Reflect.get(R, "set"); s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
