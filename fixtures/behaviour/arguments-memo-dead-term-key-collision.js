function a() { var h = ['A', 'x', '2A', '9z', 'q', 'w', '6', 'm7', 'm8', 'm9', 'm10', 'm11', '5', 'y', '3']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0;
    if (b.k === undefined) { c = arguments; b.k = true; }
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = e[f] + '!'; c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(1)) + parseInt(b(12));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 8);
console.log(b(0), b(3), b(5));
