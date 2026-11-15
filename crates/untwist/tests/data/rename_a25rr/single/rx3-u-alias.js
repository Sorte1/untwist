let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
var run = Function("return eval")();
console.log(run(s));
