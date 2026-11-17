function f(a) { return a + 1; }
function w(R = Reflect) { return R.get(R, "set"); } var s = w(); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
