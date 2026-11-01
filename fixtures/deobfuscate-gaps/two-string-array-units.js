function a1() {
  var h = ['log', 'first'];
  a1 = function () {
    return h;
  };
  return a1();
}
function b1(c, d) {
  var e = a1();
  b1 = function (f, g) {
    f = f - 0x0;
    return e[f];
  };
  return b1(c, d);
}
function a2() {
  var h = ['warn', 'second'];
  a2 = function () {
    return h;
  };
  return a2();
}
function b2(c, d) {
  var e = a2();
  b2 = function (f, g) {
    f = f - 0x0;
    return e[f];
  };
  return b2(c, d);
}
console[b1(0x0)](b1(0x1));
console[b2(0x0)](b2(0x1));
