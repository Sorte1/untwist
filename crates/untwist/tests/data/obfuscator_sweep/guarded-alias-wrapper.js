function c() {
  var o = ['TyxkK', 'hello', 'log'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  a = a - 0x10;
  var e = c();
  var h = e[a];
  return h;
}
var z = d;
function w(a) { return z(a); }
if (d(0x10) === 'TyxkK') { d = function () { return 'replaced'; }; }
console.log(w(0x11), d(0x11));
