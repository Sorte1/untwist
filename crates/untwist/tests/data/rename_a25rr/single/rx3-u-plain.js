var q = 1; function zz() { return 2; }
let lx = 3;
console.log(q + zz() + lx, Function("return 1 + 2")(), new Function("x", "return x * 2")(4));
