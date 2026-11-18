function f(a) { return a + 1; }
(function (x) { var x = this; return x; })({}).f = function () { return 99; };
console.log(f(1), f(2));
