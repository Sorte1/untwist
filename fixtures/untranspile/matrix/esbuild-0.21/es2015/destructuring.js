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
const [a, , b = 10, ...rest] = [1, 2, void 0, 4, 5];
const _a = { x: 1, q: 9, r: 8 }, { x, y: why = 2, z: { deep } = { deep: "d" } } = _a, others = __objRest(_a, ["x", "y", "z"]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point({ x: x2 = 0, y = 0 } = {}, [first, second2] = []) {
  return x2 + y + (first || 0) + (second2 || 0);
}
console.log(point({ x: 1 }, [2, 3]), point());
let m = 1;
let n = 2;
[m, n] = [n, m];
console.log(m, n);
const pairs = [[1, "one"], [2, "two"]];
for (const [num, word] of pairs) console.log(num, word);
const { length } = "hello";
const [c1, c2] = "hi";
console.log(length, c1, c2);
const data = { list: [{ id: 7, tags: ["t1", "t2"] }] };
const { list: [{ id, tags: [, second] }] } = data;
console.log(id, second);
const key = "dyn";
const { [key]: dynamic = "fallback" } = {};
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
