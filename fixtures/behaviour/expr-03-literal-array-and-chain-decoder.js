function a() { var h = ['one', 'two', 'three', 'four', 'five', 'six']; a = function () { return h; }; return a(); }
function b(c) {
  c = c - 0x10;
  var e = a();
  var t = [-0x1 + 0x2 * 0x3, 'pad', -0x0, null];
  var o = { e: e };
  return (o?.e?.[c] ?? t[0x1]) + (c === 0x4 ? t[0x0] : '');
}
console.log(b(0x10), b(0x11), b(0x12));
console.log(b(0x14), b(0x15));
