function f(a) { return a + 1; }
function h() { var { Reflect: R } = this; return R.get(R, "set"); } var s = h(); s (globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
