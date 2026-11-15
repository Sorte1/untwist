function f(a) { return a + 1; }
var k = Reflect.ownKeys(Reflect).filter(function (n) { return n === ["s", "et"].join(""); })[0]; Reflect.get(Reflect, k)(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
