"use strict";

function greet(name = "anon", greeting = "hi " + name) {
  return greeting + "!";
}
console.log(greet(), greet("bo"), greet(undefined, "yo"), greet(null));
function collect(first, ...rest) {
  return first + ":" + rest.join(",") + ":" + rest.length;
}
console.log(collect(1), collect(1, 2, 3));
function counted(a, b = 2, c) {
  return arguments.length + ":" + a + b + c;
}
console.log(counted(1), counted(1, undefined, 3), counted.length);
const arrow = (x = 1, ...ys) => x + ys.length;
console.log(arrow(), arrow(5, 6, 7), arrow.length);
let evals = 0;
function lazy(v = ++evals) {
  return v;
}
lazy(1);
lazy();
lazy();
console.log(evals);
function scoped(a, f = () => a) {
  var a = 2;
  return f();
}
console.log(scoped(1));
function withTemps(opts, size = 10, label = null) {
  var _opts$width;
  const width = (_opts$width = opts === null || opts === void 0 ? void 0 : opts.width) !== null && _opts$width !== void 0 ? _opts$width : size;
  return width + ":" + label;
}
console.log(withTemps({
  width: 3
}), withTemps(null, 4, "x"), withTemps.length);
function inlined(a, flag = false, mode = "m") {
  return [a, flag, mode].join("/");
}
console.log(inlined(1), inlined(1, true), inlined(1, undefined, "z"));
