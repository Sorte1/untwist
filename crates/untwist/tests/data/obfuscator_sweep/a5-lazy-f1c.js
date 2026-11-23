function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d() {
  var e = c();
  return (d = function (f) {
    var h = e[f === undefined ? 1 : 3];
    return h;
  }), d();
}
var z = d;
console.log(z(0x11));
console.log(z(0x11), z());
