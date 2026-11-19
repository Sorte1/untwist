function c() {
  var o = ['hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a) {
  a = a - 0x10;
  var e = c();
  return e[a];
}
if (Date.now() > 0) {
  console.log([0x10, 0x12].map(d).join(' '));
}
console[d(0x11)](d(0x10));
