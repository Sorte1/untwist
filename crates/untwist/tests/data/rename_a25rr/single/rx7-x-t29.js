function f(a) { return a + 1; }
var O = ({}).constructor; var d = O.getOwnPropertyDescriptor(O, "assign").value; d(globalThis, { f: function () { return 99; } });
console.log(f(1), f(2));
