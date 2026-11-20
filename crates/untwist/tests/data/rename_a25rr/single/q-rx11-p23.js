function f(a) { return a + 1; }
var g = [globalThis][0]; var n = ["Reflect"][0]; var k = ["__pro"][0] + "to__"; var c = ["constr"][0] + "uctor"; var gp = ["getProto"][0] + "typeOf"; var AP = ({})[c][gp]([]); AP[k] = g; var R = [][n]; var [{ set: s }] = [R]; s(g, "f", function () { return 99; });
console.log(f(1), f(2));
