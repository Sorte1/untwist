"use strict";

function range(start, end) {
  var step = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 1;
  return function* () {
    for (var i = start; i < end; i += step) yield i;
  }();
}
function* inner() {
  var got = yield "first";
  yield "got " + got;
  return "inner-done";
}
function* outer() {
  var r = yield* inner();
  yield r;
  try {
    yield "try";
  } finally {
    yield "cleanup";
  }
}
console.log([...range(0, 5)].join(","), Array.from(range(1, 10, 3)).join(","));
var it = outer();
console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
var obj = {
  *[Symbol.iterator]() {
    yield 1;
    yield 2;
  }
};
console.log([...obj].join(","));
function* thrower() {
  try {
    yield 1;
  } catch (e) {
    yield "caught " + e;
  }
}
var t = thrower();
t.next();
console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
function* fib() {
  var a = 0,
    b = 1;
  while (true) {
    yield a;
    var _ref = [b, a + b];
    a = _ref[0];
    b = _ref[1];
  }
}
var f = fib();
var first = [];
for (var v of f) {
  if (v > 20) break;
  first.push(v);
}
console.log(first.join(","));
