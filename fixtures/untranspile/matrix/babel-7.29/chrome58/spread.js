"use strict";

function sum(...nums) {
  return nums.reduce((a, b) => a + b, 0);
}
const a = [1, 2];
const b = [3, 4];
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
const holes = [, 1];
console.log([...holes].length, 0 in [...holes]);
console.log(Math.max(...a, ...b));
class Box {
  constructor(...items) {
    this.items = items;
  }
}
console.log(new Box(...a, 9).items.join(","));
const obj = {
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
