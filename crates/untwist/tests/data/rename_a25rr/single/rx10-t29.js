function f(a) { return a + 1; }
var i = 0; i = 0; globalThis.i = "Reflect"; var [{ set: s }] = [globalThis[i]]; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
