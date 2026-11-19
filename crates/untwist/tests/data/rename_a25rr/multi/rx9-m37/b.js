function f(a) { return a + 1; }
var o = {}; later(); var q = o; q.f = function () { return 99; }; console.log(f(1), f(2));
