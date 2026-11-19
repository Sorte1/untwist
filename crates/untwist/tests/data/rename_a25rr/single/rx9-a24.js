function f(a) { return a + 1; }
globalThis.eval("Symbol = { iterator: 'Function' }"); var g = [globalThis][0]; g[Symbol.iterator]("f = function () { return 99; }")();
console.log(f(1), f(2));
