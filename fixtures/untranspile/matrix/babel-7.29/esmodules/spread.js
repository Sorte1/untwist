"use strict";

function sum() {
  for (var _len = arguments.length, nums = new Array(_len), _key = 0; _key < _len; _key++) {
    nums[_key] = arguments[_key];
  }
  return nums.reduce((a, b) => a + b, 0);
}
var a = [1, 2];
var b = [3, 4];
console.log([...a, ...b, 5].join(","), sum(...a, ...b), sum(0, ...a));
function attempt(label, f) {
  try {
    console.log(label, f());
  } catch (e) {
    console.log(label, e.constructor.name);
  }
}
attempt("string", () => [..."abc"].join("-"));
attempt("set", () => [...new Set([1, 1, 2])].join(","));
var holes = [, 1];
console.log([...holes].length, 0 in [...holes]);
console.log(Math.max(...a, ...b));
class Box {
  constructor() {
    for (var _len2 = arguments.length, items = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      items[_key2] = arguments[_key2];
    }
    this.items = items;
  }
}
console.log(new Box(...a, 9).items.join(","));
var obj = {
  vals: [7, 8],
  total() {
    return sum(...this.vals);
  }
};
console.log(obj.total(), obj.vals.concat(...[[9], [10]]).join(","));
function* g() {
  yield "x";
  yield "y";
}
attempt("generator", () => [...g(), ...g()].join(""));
