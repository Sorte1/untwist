function f(a) { return a + 1; }
var d = Object.getOwnPropertyDescriptors(Reflect); var k = ["s", "et"].join(""); d[k].value(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
