function f(a) { return a + 1; }
var o = {}; later(); (function (q) { q.f = function () { return 99; }; })(o); console.log(f(1), f(2));
