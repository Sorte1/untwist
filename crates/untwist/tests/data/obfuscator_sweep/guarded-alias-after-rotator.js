function c() {
  var o = ['hello', 'log', 'world', '3', '8', 'TyxkK'];
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
(function (a, b) {
  var e = d, f = a();
  while (!![]) {
    try {
      var g = parseInt(e(0x10)) / 0x1 + parseInt(e(0x11)) / 0x2;
      if (g === b) break; else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
}(c, 0x7));
var z = d;
if (d(0x10) === '3') { d = function () { return 'replaced'; }; }
console.log(z(0x13), z(0x14));
