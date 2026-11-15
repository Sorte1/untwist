let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
var r;
r = Function("return 1")() ? typeof lx + typeof Kx + typeof q : 0;
console.log(r);
