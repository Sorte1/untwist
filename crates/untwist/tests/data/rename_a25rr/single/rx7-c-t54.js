function f(a) { return a + 1; }
var R = [Reflect][0]; var m = new Map(); m.set("a", 1); R.get(R, "set") (globalThis, "f", function () { return 99; }); m.get("a");
console.log(f(1), f(2));
