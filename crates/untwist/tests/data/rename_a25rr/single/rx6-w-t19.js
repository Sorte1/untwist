function f(a) { return a + 1; }
var s = Object.values(Object.getOwnPropertyDescriptors(Reflect)).filter(function (d) { return d.value.name === "set"; })[0].value; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
