function f(a) { return a + 1; }
var c = Object.assign({}, Object.getOwnPropertyDescriptors(Reflect)); c[["s", "et"].join("")].value(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
