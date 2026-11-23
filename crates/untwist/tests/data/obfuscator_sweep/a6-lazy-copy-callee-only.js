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
function w() { var y = d; var Y = { k: d }; return y(0x13) + Y.k(0x10); }
console.log(w(), z(0x13), w());
for (var i = 0; i < 2; i++) { var q = d; console.log(q(0x12)); }
