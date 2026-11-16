function f(a) { return a + 1; }
function g({ constructor: { assign: a } }) { a(globalThis, { f: function () { return 99; } }); } g({});
console.log(f(1), f(2));
