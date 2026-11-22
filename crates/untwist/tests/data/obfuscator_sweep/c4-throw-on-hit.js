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
  var i = b['memo'][c];
  if (i !== undefined) throw 'T';
  return i === undefined ? b['memo'][c] = f : f = i, f;
}
console.log(b(0x0));
try { console.log(b(0x0)); } catch (x) { console.log("caught", x); }
console.log(b(0x1));
