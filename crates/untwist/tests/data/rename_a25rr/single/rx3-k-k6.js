var q = 1; function zz() { return 2; }
var { a: x1, b: y1 } = globalThis; console.log(q + zz(), typeof x1, typeof y1);
