function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function d(f, g) {
    f = f - 0x10;
    d.n = (d.n || 0) + 1;
    var h = e[f];
    return h + d.n;
  }), d(a, b);
}
var z = d;
console.log(z(0x11));
console.log(z(0x11), z(0x13), d(0x12), z(0x11));
