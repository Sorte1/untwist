function f(a) { return a + 1; }
var s = Reflect.get(Object.prototype, "__defineSetter__");
console.log(f(1), f(2));
