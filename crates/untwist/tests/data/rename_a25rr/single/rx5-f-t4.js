function f(a) { return a + 1; }
Function("return Object.getOwnPropertyDescriptor")()(globalThis, "eval").value("f = function () { return 99; }");
console.log(f(1), f(2));
