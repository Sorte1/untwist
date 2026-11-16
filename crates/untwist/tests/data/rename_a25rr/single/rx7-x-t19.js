function f(a) { return a + 1; }
function* g() { yield Reflect; } for (var R of g()) R.get(R, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
