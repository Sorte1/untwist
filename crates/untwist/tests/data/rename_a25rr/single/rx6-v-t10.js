function f(a) { return a + 1; }
var k = ["s", "et"].join(""); var s = Object.getOwnPropertyDescriptor(Reflect, k).value; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
