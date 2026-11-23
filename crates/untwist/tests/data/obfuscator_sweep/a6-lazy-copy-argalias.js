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
function w(y) { y = d; return arguments[0]; }
var z = d;
console.log(z(0x11));
var t = w(1);
console.log(z(0x13));
var u = w(1);
console.log(t === u);
