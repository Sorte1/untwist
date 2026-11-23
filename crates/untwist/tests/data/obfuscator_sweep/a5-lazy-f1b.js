function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function (f, g, ...r) {
    f = f - 0x10;
    var h = e[f];
    return h + r.length;
  }), d(a, b);
}
var z = d;
console.log(z(0x11, 1, 2));
console.log(z(0x11, 1, 2), z(0x13, 1, 2, 3));
