var q = 1; function zz() { return 2; }
let lx = 3; const cx = 4; class Kx {}
var s = "typeof q + typeof zz + typeof lx + typeof cx + typeof Kx";
var r;
var o = { e: eval }; r = o.e(s);
console.log(r);
