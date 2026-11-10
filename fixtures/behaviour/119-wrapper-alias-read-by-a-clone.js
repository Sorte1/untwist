function arr() {
  var h = ['log', 'hello', 'world', 'abc', 'def'];
  arr = function () { return h; };
  return arr();
}
function dec(a, b) {
  a = a - 100;
  var e = arr();
  return e[a];
}
function alias(a, b) { return dec(b - 938, a); }
function alias2(a, b) { return dec(b - 938, a); }
var z = alias;
console[alias(0, 1038)](alias2(z, 1040), z(7, 1039));
