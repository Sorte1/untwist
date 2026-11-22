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
    return i === undefined ? m[f] = h : h = i + '!', h;
  }), b(c, d);
}
var z = b;
for (var n = 0; n < 2; n++) console.log(z(0x0));
