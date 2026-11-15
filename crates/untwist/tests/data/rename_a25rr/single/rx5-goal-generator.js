function f(a) { return a + 1; }
var GF = (function* () {}).constructor;
var it = GF("f = function () { return 99; }; yield 1")();
it.next();
console.log(f(1), f(2));
