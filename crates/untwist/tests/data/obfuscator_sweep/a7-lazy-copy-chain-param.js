function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function (f, g) {
    f = f - 0x10;
    var h = e[f];
    return h;
  }), d(a, b);
}
var z = d;
console.log(z(0x11));
var t = d; function w(y) { y = t; return arguments[0]; }
console.log(z(0x13));
var u = d; function w2(y) { y = u; return arguments[0]; }
console.log(w(1) === w2(1));
