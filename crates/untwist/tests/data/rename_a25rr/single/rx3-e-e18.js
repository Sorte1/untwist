var q = 1; function zz() { return 2; }
let lx = 3; const cx = 4; class Kx {}
var s = "typeof q + typeof zz + typeof lx + typeof cx + typeof Kx";
var r;
r = (async function () {}).constructor === 0 ? 0 : (function* () {}).constructor("return typeof q + typeof Kx")().next().value;
console.log(r);
