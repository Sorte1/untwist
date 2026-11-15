var q = 1; function zz() { return 2; }
let lx = 3; const cx = 4; class Kx {}
var s = "typeof q + typeof zz + typeof lx + typeof cx + typeof Kx";
var r;
r = self0().setTimeout ? typeof q : 0; function self0() { return globalThis; }
console.log(r);
