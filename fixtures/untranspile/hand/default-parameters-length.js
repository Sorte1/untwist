function loose(a, b) {
  if (b === void 0) {
    b = a + 1;
  }
  return a + b;
}
function looseAfterDefault(a, b, c) {
  var x = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
  if (c === void 0) {
    c = 3;
  }
  return x + c;
}
function plain() {
  var a = arguments.length > 0 ? arguments[0] : undefined;
  return a;
}
function gap(a) {
  var c = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 9;
  return a + c;
}
function gapPlain(a) {
  var c = arguments.length > 2 ? arguments[2] : undefined;
  return a + "" + c;
}
function kept() {
  var a = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 2;
  return a;
}
var arrow = function (a, b) {
  if (b === void 0) {
    b = 5;
  }
  return a * b;
};
console.log(loose.length, loose(1), looseAfterDefault.length, looseAfterDefault());
console.log(plain.length, plain(4), gap.length, gap(1), gapPlain.length, gapPlain(1, 2, 3));
console.log(kept.length, kept(), arrow.length, arrow(2));
