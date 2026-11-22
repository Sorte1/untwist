function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
var hook;
function d(a, b) {
  a = a - 0x10;
  var e = c();
  var h = e[a];
  hook = function () { d = function () { return 'replaced'; }; };
  return h;
}
var z = d;
console.log(z(0x11));
hook();
console.log(z(0x11), d(0x11));
