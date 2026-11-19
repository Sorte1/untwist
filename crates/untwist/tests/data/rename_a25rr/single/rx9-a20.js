function f(a) { return a + 1; }
let o = {}; Function("\\u006f = globalThis")(); o.f = function () { return 99; };
console.log(f(1), f(2));
