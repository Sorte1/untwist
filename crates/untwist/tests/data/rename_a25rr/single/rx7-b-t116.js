function f(a) { return a + 1; }
function w({ set: s } = Reflect) { s (globalThis, "f", function () { return 99; }); } w();
console.log(f(1), f(2));
