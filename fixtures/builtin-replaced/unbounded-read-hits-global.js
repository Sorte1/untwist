function a() { var h = ['log', '6', 'Hello World!', 'charAt']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var name = ['cache', 'Key'].join(''); return e[f] + (this[name] || ''); }; return b(c, d); }
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
globalThis.cacheKey = '!';
console.log(b(0x1));
