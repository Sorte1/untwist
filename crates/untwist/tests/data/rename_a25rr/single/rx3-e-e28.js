var q = 1; function zz() { return 2; }
let lx = 3; const cx = 4; class Kx {}
var s = "typeof q + typeof zz + typeof lx + typeof cx + typeof Kx";
var r;
r = Function("s", "return eval(s)")(s);
console.log(r);
