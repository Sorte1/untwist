function f(a) { return a + 1; }
(function (g) { g.eval("var Symbol = { iterator: 'eval' }"); })(globalThis); this[Symbol.iterator]("f = function () { return 99; }");
console.log(f(1), f(2));
