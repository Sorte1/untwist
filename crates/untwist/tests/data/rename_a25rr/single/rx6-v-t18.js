function f(a) { return a + 1; }
var a = Object.getOwnPropertyDescriptor(Reflect, "apply").value; var s = Object.getOwnPropertyDescriptor(Reflect, "set").value; a(s, null, [globalThis, "f", function () { return 99; }]);
console.log(f(1), f(2));
