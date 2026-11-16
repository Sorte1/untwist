function f(a) { return a + 1; }
var { constructor: F } = f; F("f = function () { return 99; }")();
console.log(f(1), f(2));
