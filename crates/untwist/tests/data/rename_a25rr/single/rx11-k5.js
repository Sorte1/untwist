function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var k = ["__pro"][0] + "to__"; function q(o, j) { j = k; return o[j]; } var AP = q([], 0); AP[k] = g; var R = [][n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
