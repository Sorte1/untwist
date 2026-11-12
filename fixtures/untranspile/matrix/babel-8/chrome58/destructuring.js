function _objectWithoutProperties(t, r) { if (null == t) return {}; var o, n, i = _objectWithoutPropertiesLoose(t, r); if (Object.getOwnPropertySymbols) { var l = Object.getOwnPropertySymbols(t); for (n = 0; n < l.length; n++) o = l[n], -1 === r.indexOf(o) && {}.propertyIsEnumerable.call(t, o) && (i[o] = t[o]); } return i; }
function _objectWithoutPropertiesLoose(r, e) { if (null == r) return {}; var t = {}; for (var n in r) if ({}.hasOwnProperty.call(r, n)) { if (-1 !== e.indexOf(n)) continue; t[n] = r[n]; } return t; }
const [a,, b = 10, ...rest] = [1, 2, undefined, 4, 5];
const _x$q$r = {
    x: 1,
    q: 9,
    r: 8
  },
  {
    x,
    y: why = 2,
    z: {
      deep
    } = {
      deep: "d"
    }
  } = _x$q$r,
  others = _objectWithoutProperties(_x$q$r, ["x", "y", "z"]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point({
  x = 0,
  y = 0
} = {}, [first, second] = []) {
  return x + y + (first || 0) + (second || 0);
}
console.log(point({
  x: 1
}, [2, 3]), point());
let m = 1;
let n = 2;
[m, n] = [n, m];
console.log(m, n);
const pairs = [[1, "one"], [2, "two"]];
for (const [num, word] of pairs) console.log(num, word);
const {
  length
} = "hello";
const [c1, c2] = "hi";
console.log(length, c1, c2);
const data = {
  list: [{
    id: 7,
    tags: ["t1", "t2"]
  }]
};
const {
  list: [{
    id,
    tags: [, second]
  }]
} = data;
console.log(id, second);
const key = "dyn";
const {
  [key]: dynamic = "fallback"
} = {};
console.log(dynamic);
function swap([p, q]) {
  return [q, p];
}
console.log(swap([1, 2]).join(","));
const [g1, g2] = function* () {
  yield "a";
  yield "b";
  yield "c";
}();
console.log(g1, g2);
