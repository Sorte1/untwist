let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
var r;
r = Function("return setTimeout")() ? (0, eval)(s) : 0;
console.log(r);
