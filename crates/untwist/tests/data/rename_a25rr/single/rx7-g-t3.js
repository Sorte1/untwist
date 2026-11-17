function f(a) { return a + 1; }
var g = this; Object.assign(g, { f: function () { return 99; } });
console.log(f(1), f(2));
