function f(a) { return a + 1; }
var k = ["s", "et"].join(""); var s; ({ [k]: s } = Reflect); s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
