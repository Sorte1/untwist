function f(a) { return a + 1; }
var o = { ...Object.getOwnPropertyDescriptors(Object) }; o[["ass", "ign"].join("")].value(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
