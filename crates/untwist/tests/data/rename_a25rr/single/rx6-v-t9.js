function f(a) { return a + 1; }
var k = ["s", "et"].join(""); var s = Reflect.get(Reflect, k); s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
