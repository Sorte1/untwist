function c() {
  var o = ['TyxkK', 'hello', 'log'];
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
var z = d; var y = z;
if (d(0x10) === 'TyxkK') { d = function () { return 'replaced'; }; }
console.log(y(0x11));
