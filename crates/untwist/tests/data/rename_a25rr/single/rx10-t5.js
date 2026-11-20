function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var w = { bind: function () { return g; } }; var k = ["getPrototypeO"][0] + "f"; var P = ({}).constructor[k]([]); Object.setPrototypeOf(P, w.bind()); var R = [][n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
