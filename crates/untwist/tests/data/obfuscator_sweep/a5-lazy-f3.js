function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, a) {
  var e = c();
  return (d = function (f, g) {
    f = f - 0x10;
    var h = e[f];
    return h;
  }), d(a, a);
}
var z = d;
console.log(z(0x11, 0x13));
console.log(z(0x11, 0x13), z(0x12, 0x11));
