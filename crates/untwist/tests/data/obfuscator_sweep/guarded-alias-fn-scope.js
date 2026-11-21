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
(function () { var d2 = d; function k() { if (d2(0x10) === 'TyxkK') { d = function () { return 'r2'; }; } } console.log(d2(0x11), d(0x11)); k(); console.log(d2(0x11), d(0x11)); })();
console.log(d(0x11), d(0x12), d(0x13));
