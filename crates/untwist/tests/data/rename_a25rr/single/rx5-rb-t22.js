function f(a) { return a + 1; }
var fn = function () { return 99; };
[fn].forEach(Reflect.set.bind(null, globalThis, "f"));
console.log(f(1), f(2));
