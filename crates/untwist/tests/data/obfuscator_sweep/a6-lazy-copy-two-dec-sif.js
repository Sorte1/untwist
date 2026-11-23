(function () {
'use strict';
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
function d2(a, b) {
  var e = c();
  return (d2 = function (f, g) {
    f = f - 0x10;
    var h = e[f];
    return h;
  }), d2(a, b);
}
var z = d, y = d2;
console.log(z(0x11), y(0x11));
var t = d2;
console.log(z(0x13), y(0x13));
var u = d2;
console.log(t === u);
})();
