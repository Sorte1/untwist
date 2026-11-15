function f(a) { return a + 1; }
var s = Object.getOwnPropertyDescriptor(Object, "defineProperty").value; s(globalThis, "f", { value: function () { return 99; } });
console.log(f(1), f(2));
