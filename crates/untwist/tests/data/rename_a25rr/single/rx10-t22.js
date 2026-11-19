function f(a) { return a + 1; }
var i = "zz"; globalThis.i = "Reflect"; var [{ set: s }] = [globalThis[i]]; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
