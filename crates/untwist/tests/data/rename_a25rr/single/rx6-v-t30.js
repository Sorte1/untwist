function f(a) { return a + 1; }
var s = structuredClone ? Object.getOwnPropertyDescriptor(Reflect, "set").value : 0; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
