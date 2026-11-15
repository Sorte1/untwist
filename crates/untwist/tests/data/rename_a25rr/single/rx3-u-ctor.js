let lx = 3; class Kx {}
var q = 1;
var s = "return typeof lx + typeof Kx + typeof q";
console.log(Function("return [].map.constructor")()(s)());
