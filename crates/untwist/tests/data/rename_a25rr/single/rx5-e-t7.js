function f(a) { return a + 1; }
Function("return Reflect.get(function () {}, 'constructor')")()("return globalThis")().f = function () { return 99; };
console.log(f(1), f(2));
