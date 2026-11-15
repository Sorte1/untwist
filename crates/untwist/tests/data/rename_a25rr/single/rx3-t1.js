var q = 1; function zz() { return 2; }
var g = Function("return this")();
console.log(g.q, typeof g.zz);
