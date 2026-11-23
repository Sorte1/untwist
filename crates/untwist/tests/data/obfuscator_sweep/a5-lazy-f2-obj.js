function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function me(f, g) {
    f = f - 0x10;
    me.n = (me.n || 0) + 1;
    var h = e[f];
    return h + me.n;
  }), d(a, b);
}
var A = { k: d };
console.log(A.k(0x11));
console.log(A.k(0x11), A.k(0x13));
