function a() { var h = ['x', 'alpha', 'beta', 'gamma', 'delta']; a = function () { return h; }; return a(); }
function b(c, d) {
  c = c - 0x1;
  var e = a();
  var r = e[arguments[0]];
  r ??= `${d}`;
  return { v: r }.v;
}
console.log(b(0x2) + b(0x3));
console.log(b(0x4), b(0x5));
