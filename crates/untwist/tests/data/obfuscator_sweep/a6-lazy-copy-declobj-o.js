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
var Z = { k: d };
console.log(Z.k(0x11));
var t = d;
console.log(Z.k(0x13));
var B = { k: d };
console.log(t === B.k);
