const d = b;
(function (c, e) {
  const f = b, g = c();
  while (!![]) {
    try {
      const h = parseInt(f(0x0));
      if (h === e) break;
      else g['push'](g['shift']());
    } catch (i) {
      g['push'](g['shift']());
    }
  }
}(a, 0x3));
function b(c, e) {
  const f = a();
  return b = function (g, h) {
    g = g - 0x0;
    let i = f[g];
    return i;
  }, b(c, e);
}
function show(x) {
  return d(0x1) + x;
}
console.log(d(0x2), show('!'), b(0x3));
function a() {
  const j = ['1', '3', 'hi', 'there', 'x'];
  a = function () {
    return j;
  };
  return a();
}
