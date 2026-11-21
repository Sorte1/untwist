function a() {
  var d = ['alpha', 'beta', 'gamma'];
  a = function () { return d; };
  return a();
}
function b(c, d) {
  c = c - 0x0;
  var e = a();
  var f = e[c];
  if (b['init'] === undefined) {
    b['memo'] = {}, b['init'] = !![];
  }
  var k = c % 2;
  var i = b['memo'][k];
  return i === undefined ? (b['memo'][k] = f, c === 0 ? (function () { throw 'T'; })() : 0) : f = i, f;
}
try { console.log(b(0x0)); } catch (x) { console.log("caught", x); }
console.log(b(0x2));
console.log(b(0x1));
