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
var A = { k: d };
console.log(A.k(0x11));
c = function () { return ['R0', 'R1', 'R2', 'R3']; };
console.log(A.k(0x11), A.k(0x13));
