function f(a) { return a + 1; }
let o = {}; later(); o.f = function () { return 99; }; console.log(f(1), f(2));
