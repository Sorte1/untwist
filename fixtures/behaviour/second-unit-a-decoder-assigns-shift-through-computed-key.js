(function (b, e) {
  var k = d, f = b();
  while (!![]) {
    try {
      var g = parseInt(k(0x63)) / 0x1 + parseInt(k(0x64)) / 0x2;
      if (g === e) break;
      else f['push'](f['shift']());
    } catch (h) {
      f['push'](f['shift']());
    }
  }
})(c, 0x1d);
function c() {
  var o = ['12alpha', '34beta', 'log', 'first'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function (f, g) {
    f = f - 0x61;
    var h = e[f];
    if (f === 0x1) {
      Object['assi' + 'gn'](Array['prototype'], { shift: function () { return this['pop'](); } });
    }
    return h;
  }), d(a, b);
}
console[d(0x61)](d(0x62));
(function (b, e) {
  var k = q, f = b();
  var tries = 0x0;
  while (!![]) {
    try {
      var g = parseInt(k(0x1f6)) / 0x2 + parseInt(k(0x1f7)) / 0x3;
      if (g === e) break;
      else f['push'](f['shift']());
    } catch (h) {
      f['push'](f['shift']());
    }
    if (tries++ > 0x3e8) break;
  }
})(p, 0x36);
function p() {
  var o = ['second', '56gamma', '78delta', 'log'];
  p = function () { return o; };
  return p();
}
function q(a, b) {
  var e = p();
  return (q = function (f, g) {
    f = f - 0x1f4;
    var h = e[f];
    return h;
  }), q(a, b);
}
console['log'](q(0x1f5));
