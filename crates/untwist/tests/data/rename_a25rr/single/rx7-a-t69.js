function f(a) { return a + 1; }
var F = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), "constructor").value; F("f = function () { return 99; }")();
console.log(f(1), f(2));
