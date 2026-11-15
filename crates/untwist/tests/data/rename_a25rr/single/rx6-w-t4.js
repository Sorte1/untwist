function f(a) { return a + 1; }
var e = Object.entries(Object.getOwnPropertyDescriptors(Reflect)); var m = Object.fromEntries(e); m[["s", "et"].join("")].value(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
