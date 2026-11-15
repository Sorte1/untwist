function f(a) { return a + 1; }
var { ...r } = Object.getOwnPropertyDescriptors(Reflect); r.set.value(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
