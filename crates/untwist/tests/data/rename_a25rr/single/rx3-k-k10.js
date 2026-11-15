var q = 1; function zz() { return 2; }
console.log(q + zz(), "a,b".split(",").map(function (k) { return k in globalThis; }).join());
