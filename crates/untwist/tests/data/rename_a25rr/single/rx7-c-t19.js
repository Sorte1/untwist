function f(a) { return a + 1; }
var { constructor: { assign: a } } = {}; a(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
