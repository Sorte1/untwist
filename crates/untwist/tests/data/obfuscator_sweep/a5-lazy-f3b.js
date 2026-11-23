function a() {
  var d = ['alpha', 'beta', 'gamma', 'delta', 'length', '0', '1'];
  a = function () { return d; };
  return a();
}
function b(c, c) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0x0;
    var h = e[f];
    if (b['init'] === undefined) {
      b['memo'] = {}, b['init'] = !![];
    }
    var k = f, i = b['memo'][k];
    return i === undefined ? (h = h, b['memo'][k] = h) : h = i, h;
  }), b(c, c);
}
var z = b;
console.log(z(0x1, 1));
console.log(z(0x1, 2), z(0x2, 1), z(0x0, 0), z(0x0, 1), z(0x1, 0));
