function f(a) { return a + 1; }
[Reflect.get(function () {}, "constructor")][0]("f = function () { return 99; }")();
console.log(f(1), f(2));
