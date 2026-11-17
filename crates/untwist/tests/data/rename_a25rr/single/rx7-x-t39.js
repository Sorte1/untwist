function f(a) { return a + 1; }
var R = [Object][0]; R.getOwnPropertyDescriptor(R, "assign").value(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
