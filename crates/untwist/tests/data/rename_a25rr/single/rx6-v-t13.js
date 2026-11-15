function f(a) { return a + 1; }
var g = Object.getOwnPropertyDescriptor(Reflect, "get").value; var e = g(globalThis, ["ev", "al"].join("")); e("f = function () { return 99; }");
console.log(f(1), f(2));
