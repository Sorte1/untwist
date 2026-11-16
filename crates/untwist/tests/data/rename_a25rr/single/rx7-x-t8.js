function f(a) { return a + 1; }
class K extends ({}).constructor {} var { assign: a } = K; a(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
