var q = 1; function zz() { return 2; }
console.log(q + zz(), Object.keys({ a: 1, b: 2 }).map(function (k) { return typeof globalThis[k]; }).join());
