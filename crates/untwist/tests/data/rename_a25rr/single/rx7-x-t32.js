function f(a) { return a + 1; }
var R = Reflect.get(globalThis, "Reflect"); [R][0].get(R, "set") (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
