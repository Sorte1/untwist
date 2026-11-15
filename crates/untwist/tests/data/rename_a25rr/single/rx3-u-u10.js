let lx = 3; class Kx {}
var q = 1;
var s = "typeof lx + typeof Kx + typeof q";
var r;
r = Reflect.construct(Function, ["return eval"])()(s);
console.log(r);
