function f(a) { return a + 1; }
var o = Object.create(({}).constructor); var { assign: a } = o; a(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
