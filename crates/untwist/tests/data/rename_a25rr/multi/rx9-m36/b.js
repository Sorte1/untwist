function f(a) { return a + 1; }
(function () { later(); })(); var o = {}; [1].forEach(later); o.f = function () { return 99; }; console.log(f(1), f(2));
