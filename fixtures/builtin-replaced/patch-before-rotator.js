function a() { var h = ['log', '6', 'Hello World!', 'charAt']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var keep = Array.prototype.shift;
Array.prototype.shift = function () { keep.call(this); return keep.call(this); };
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
console.log(b(0x1));
