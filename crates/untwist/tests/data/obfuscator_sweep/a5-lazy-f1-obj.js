function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a) {
  var e = c();
  return (d = function (f, g) {
    f = f - 0x10;
    var h = e[f];
    return g === undefined ? h : h + '!';
  }), d(a);
}
var A = { k: d };
console.log(A.k(0x11, 1));
console.log(A.k(0x11, 1), A.k(0x13, 1));
