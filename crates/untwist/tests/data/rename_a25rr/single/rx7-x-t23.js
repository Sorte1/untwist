function f(a) { return a + 1; }
var { a = Reflect } = {}; a.get(a, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
