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
var seen = [];
console.log(z(0x11));
seen.push(d);
console.log(z(0x13));
seen.push(d);
console.log(seen[0] === seen[1]);
