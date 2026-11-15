var q = 1; function zz() { return 2; }
let lx = 3; const cx = 4; class Kx {}
var s = "typeof q + typeof zz + typeof lx + typeof cx + typeof Kx";
var r;
r = new Function("return typeof q + typeof zz + typeof lx")();
console.log(r);
