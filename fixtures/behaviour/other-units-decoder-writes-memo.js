function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
function p() { var h = ['u0', 'u1', 'u2']; p = function () { return h; }; return p(); }
function q(i) { var e = p(); if (b.c) { b.c[1] = 'EVIL'; } return e[i]; }
console.log(b(0)); console.log(q(0)); console.log(b(1));
