let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
var r;
var f = Function("return eval"); var e2 = f(); r = e2(s);
console.log(r);
