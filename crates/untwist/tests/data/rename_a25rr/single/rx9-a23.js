function f(a) { return a + 1; }
globalThis.eval("var Symbol = { iterator: 'eval' }"); globalThis[Symbol.iterator]("f = function () { return 99; }");
console.log(f(1), f(2));
