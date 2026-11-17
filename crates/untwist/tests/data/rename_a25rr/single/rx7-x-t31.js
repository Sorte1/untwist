function f(a) { return a + 1; }
var R = Object.getOwnPropertyDescriptor(globalThis, "Reflect").value; var s = R.get(R, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
