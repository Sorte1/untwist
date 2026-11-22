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
  var k = c % 0x2;
  var i = b['memo'][k];
  if (i === undefined && c === 0x0) throw 'T';
  return i === undefined ? b['memo'][k] = f : f = i, f;
}
for (var n = 0; n < 2; n++) {
  try { console.log(b(0x0)); } catch (x) { console.log("caught", x); }
  console.log(b(0x2));
}
