var q = 1; function zz() { return 2; }
var g = Function("return (function () {}.constructor(\"return this\")())")();
console.log(g.q, typeof g.zz);
