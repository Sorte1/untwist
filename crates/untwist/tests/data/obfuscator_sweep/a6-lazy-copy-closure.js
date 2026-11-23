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
var t, u;
function same() { return t === u; }
var z = d;
console.log(z(0x11));
t = d;
console.log(z(0x13));
u = d;
console.log(same());
