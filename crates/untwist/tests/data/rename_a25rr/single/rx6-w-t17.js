function f(a) { return a + 1; }
var d = Object.getOwnPropertyDescriptor(Reflect, "has").value; console.log(d({}, "x"));
console.log(f(1), f(2));
