function g(a, b) {
  var x;
  a = function () { return x; };
  b = x + 1;
  x = 5;
  return [a(), b];
}
console.log(g(1, 2));
function h(a) {
  var x;
  a = { v: x };
  x = "s";
  return a.v;
}
console.log(h(1));
