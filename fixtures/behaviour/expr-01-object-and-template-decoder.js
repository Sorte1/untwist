function a() { var h = ['log', 'Hello', 'World', 'console', 'unused']; a = function () { return h; }; return a(); }
function b(c, d) {
  c = c - 0x0;
  var e = a();
  var o = { s: e[c], t: `${e[c]}!`, [`k${c}`]: c };
  var k = o?.s;
  k ||= 'none';
  delete o.t;
  return k;
}
console[b(0x0)](b(0x1) + ' ' + b(0x2));
console[b(0x0)](b(0x3));
