var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __objRest = (source, exclude) => {
  var target = {};
  for (var prop in source)
    if (__hasOwnProp.call(source, prop) && exclude.indexOf(prop) < 0)
      target[prop] = source[prop];
  if (source != null && __getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(source)) {
      if (exclude.indexOf(prop) < 0 && __propIsEnum.call(source, prop))
        target[prop] = source[prop];
    }
  return target;
};
const sym = /* @__PURE__ */ Symbol.for("s");
const src = { a: 1, b: 2, c: 3, [sym]: 4 };
const _a = src, { a } = _a, others = __objRest(_a, ["a"]);
console.log(a, Object.keys(others).join(","), Object.getOwnPropertySymbols(others).length);
function pick(_b) {
  var _c = _b, { b } = _c, rest = __objRest(_c, ["b"]);
  return b + ":" + Object.keys(rest).join(",");
}
console.log(pick(src));
const _d = src, { ["c"]: c, b: bee } = _d, remaining = __objRest(_d, ["c", "b"]);
console.log(c, bee, Object.keys(remaining).join(","));
const [head, ...tail] = [1, 2, 3, 4];
console.log(head, tail.join(","));
const [k1, k2] = (/* @__PURE__ */ new Map([["k", "v"], ["k2", "v2"]])).keys();
console.log(k1, k2);
const reads = [];
const watched = {
  get x() {
    reads.push("x");
    return 1;
  },
  get y() {
    reads.push("y");
    return 2;
  },
  z: 3
};
const _e = watched, { y } = _e, fromWatched = __objRest(_e, ["y"]);
console.log(y, Object.keys(fromWatched).join(","), reads.join(","));
const box = { p: 1, q: 2 };
var p1 = box.p;
var q1 = box.q;
console.log(p1, q1);
try {
  const { only } = null;
  console.log(only);
} catch (e) {
  console.log(e.constructor.name);
}
const [first, second] = "xyz";
console.log(first, second);
const counted = {
  [Symbol.iterator]() {
    let n = 0;
    return {
      next() {
        n += 1;
        return { value: n, done: n > 5 };
      },
      return() {
        console.log("closed");
        return {};
      }
    };
  }
};
const [one, two] = counted;
console.log(one, two);
