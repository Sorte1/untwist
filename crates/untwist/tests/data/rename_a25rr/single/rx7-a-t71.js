function f(a) { return a + 1; }
var d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(function () {}), "constructor"); var { value } = d; value("f = function () { return 99; }")();
console.log(f(1), f(2));
