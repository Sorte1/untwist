function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function (f, g) {
    if (g === 'reset') { d = function () { return 'replaced'; }; return; }
    f = f - 0x10;
    var h = e[f];
    return h;
  }), d(a, b);
}
var z = d;
console.log(z(0x11));
var y = d;
d(0, 'reset');
console.log(z(0x11), y(0x11), d(0x11));
