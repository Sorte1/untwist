function a() { var h = ['log', '6', 'Hello World!', 'charAt']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var name = ['char', 'At'].join(''); var s = e[f]; return s.length > 0 ? s[name](0) + s.slice(1) : s; }; return b(c, d); }
(function (c, e) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) / 0x1;
      if (g === e) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0x6);
String.prototype.charAt = function () { return '?'; };
console.log(b(0x1));
