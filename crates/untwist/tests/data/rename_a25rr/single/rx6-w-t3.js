function f(a) { return a + 1; }
var v = Object.values(Object.getOwnPropertyDescriptors(Reflect)); v.forEach(function (d) { if (d.value && d.value.length === 3 && d.value.name === "set") d.value(globalThis, "f", function () { return 99; }); });
console.log(f(1), f(2));
