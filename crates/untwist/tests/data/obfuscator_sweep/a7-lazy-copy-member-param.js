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
var A = { k: d }; function w(y) { var y = A.k; return arguments[0]; }
console.log(z(0x13));
var B = { k: d }; function w2(y) { var y = B.k; return arguments[0]; }
console.log(w(1) === w2(1));
