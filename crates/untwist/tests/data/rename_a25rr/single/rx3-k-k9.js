var q = 1; function zz() { return 2; }
console.log(q + zz(), JSON.parse('["a","b"]').map(function (k) { return typeof globalThis[k]; }).join());
