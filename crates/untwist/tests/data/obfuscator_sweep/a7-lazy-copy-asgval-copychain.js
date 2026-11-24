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
var z = d;
console.log(z(0x11));
var t = d, y1; var r1 = [y1 = t];
console.log(z(0x13));
var u = d, y2; var r2 = [y2 = u];
console.log(r1[0] === r2[0]);
