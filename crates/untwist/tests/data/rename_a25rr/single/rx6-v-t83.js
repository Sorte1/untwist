function f(a) { return a + 1; }
var s = Object.getOwnPropertyDescriptor(Reflect, "set").value; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
