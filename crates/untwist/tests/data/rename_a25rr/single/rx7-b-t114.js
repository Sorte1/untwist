function f(a) { return a + 1; }
(function ({ assign: a }) { a(globalThis, { f: function () { return 99; } }); })(Object);
console.log(f(1), f(2));
