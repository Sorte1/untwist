function f(a) { return a + 1; }
var s = Object.getOwnPropertyDescriptor(globalThis, "Reflect").value; var w = s.set; w(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
