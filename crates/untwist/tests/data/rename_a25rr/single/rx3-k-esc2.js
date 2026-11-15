var q = 1; function zz() { return 2; }
var g = Function("return this")();
console.log(q + zz(), typeof g["\x61"], typeof g["b"]);
