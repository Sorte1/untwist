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
if (d(0x10) !== 'TyxkK') {
  console.log(typeof d);
} else {
  console[d(0x12)](d(0x11));
}
