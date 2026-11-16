function f(a) { return a + 1; }
var s = Object.getOwnPropertyDescriptor(Function.prototype, "constructor").value; s("f = function () { return 99; }")();
console.log(f(1), f(2));
