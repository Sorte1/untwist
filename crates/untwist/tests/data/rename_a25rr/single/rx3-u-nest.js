let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
console.log(Function("return Function('return eval')")()()(s));
