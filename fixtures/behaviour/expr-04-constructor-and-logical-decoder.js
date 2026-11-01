function a() { var h = ['hello', 'from', 'some', 'constructed', 'decoder']; a = function () { return h; }; return a(); }
function b(c) {
  var e = a();
  var D = function (s) { this.s = s; };
  var x = new D(e[c]);
  x.s &&= x.s + '!';
  x.n ||= `${c}`;
  return x.s + x.n;
}
console.log(b(0) + ' ' + b(1));
console.log(b(3), b(4));
