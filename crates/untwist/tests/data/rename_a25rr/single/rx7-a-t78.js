function f(a) { return a + 1; }
var F = Reflect.get(Reflect.get(f, "__proto__"), "constructor"); F("f = function () { return 99; }")();
console.log(f(1), f(2));
