function f(a) { return a + 1; }
var fn = function () { return 99; };
Object.assign.bind(Object, globalThis)({ f: fn });
console.log(f(1), f(2));
