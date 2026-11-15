function f(a) { return a + 1; }
var m = new Map(Object.entries(Object.getOwnPropertyDescriptors(Reflect))); m.get("set").value(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
