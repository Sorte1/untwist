function f(a) { return a + 1; }
var o = {}; Object.setPrototypeOf(o, Reflect); var { set: s } = o; s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
