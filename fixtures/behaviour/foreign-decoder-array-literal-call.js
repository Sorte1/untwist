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
function p() { var h = ['u0', 'u1', 'u2']; p = function () { return h; }; return p(); }
function q(i) { var e = p(); var z = [b][0](0, 'y'); return e[i]; }
console.log(q(0)); console.log(b(0, 'x')); console.log(b(1, 'x'));
