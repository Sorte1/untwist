const sym = /* @__PURE__ */ Symbol.for("s");
const src = { a: 1, b: 2, c: 3, [sym]: 4 };
const { a, ...others } = src;
console.log(a, Object.keys(others).join(","), Object.getOwnPropertySymbols(others).length);
function pick({ b, ...rest }) {
  return b + ":" + Object.keys(rest).join(",");
}
console.log(pick(src));
const { ["c"]: c, b: bee, ...remaining } = src;
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
const { y, ...fromWatched } = watched;
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
