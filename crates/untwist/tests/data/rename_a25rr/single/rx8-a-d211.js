function f(a) { return a + 1; }
var o = {}; setTimeout("zz = 1"); globalThis.o = globalThis; o.f = function () { return 99; };
console.log(f(1), f(2));
