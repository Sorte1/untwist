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
function q() { return 'x'; }
var z = d;
if (q() === 'x') { d = function (a) { return z(a) + '!'; }; }
console.log(d(0x11));
