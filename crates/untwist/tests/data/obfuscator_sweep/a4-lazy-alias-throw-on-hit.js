function a() {
  var d = ['alpha', 'beta', 'gamma'];
  a = function () { return d; };
  return a();
}
function b(c, d) {
  var e = a();
  var m = {};
  return (b = function (f, g) {
    f = f - 0x0;
    var h = e[f];
    var i = m[f];
    if (i !== undefined) throw 'T';
    return m[f] = h, h;
  }), b(c, d);
}
var z = b;
console.log(z(0x0));
console.log(z(0x0));
