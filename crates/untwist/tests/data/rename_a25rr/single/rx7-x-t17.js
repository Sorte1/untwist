function f(a) { return a + 1; }
var m = new Map([["r", Reflect]]); var R = m.get("r"); var s = R.get(R, "set"); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
