function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { b.r = function (x, y) { return x + y; }; c = arguments; b.k = true; }
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = b.r(e[f], g); c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
function p() { var h = ['x', '3', 'u2', 'u3']; p = function () { return h; }; return p(); }
function q(c, d) {
  var e = p();
  return (q = function (f, g) {
    f = f - 0;
    return e[f];
  }), q(c, d);
}
(function (c, d) {
  var e = c();
  var z = b(0, 'y');
  while (!![]) {
    try {
      var f = parseInt(q(0));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(p, 3);
console.log(q(0), q(1), q(2), q(3)); console.log(b(0, 'x')); console.log(b(1, 'x'));
