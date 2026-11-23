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
var S = [];
for (var i = 0; i < 1; i++) { var q = d; S.push(q); }
console.log(z(0x13));
for (var i = 0; i < 1; i++) { var q = d; S.push(q); }
console.log(S[0] === S[1]);
