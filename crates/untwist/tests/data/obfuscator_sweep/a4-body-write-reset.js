function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  if (a === 'reset') { d = function () { return 'replaced'; }; return; }
  a = a - 0x10;
  var e = c();
  var h = e[a];
  return h;
}
var z = d;
console.log(z(0x11));
d('reset');
console.log(z(0x11), d(0x11));
