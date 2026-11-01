const [p, , q = 7, ...r] = [1, 2, undefined, 4, 5];
console.log(p, q, r.join(","));
const [m, n] = new Set(["s1", "s2", "s3"]);
console.log(m, n);
const { a: { b: deep } = { b: "dflt" }, c = 3, ["k" + 1]: computed } = { c: undefined, k1: "kv" };
console.log(deep, c, computed);
function f({ x, y = 2 } = {}, [z] = [9]) {
  return [x, y, z].join("/");
}
console.log(f(), f({ x: 1 }, [5]), f.length);
let u, v;
[u, v] = [v, u] = [1, 2];
console.log(u, v);
for (const [key, value] of Object.entries({ e1: 1, e2: 2 })) console.log(key, value);
const order = [];
const src = {
  get first() { order.push("first"); return 1; },
  get second() { order.push("second"); return 2; },
};
const { second, first } = src;
console.log(order.join(","), first, second);
try {
  const [t] = null;
  console.log(t);
} catch (e) {
  console.log(e.constructor.name);
}
try {
  const { t2 } = undefined;
  console.log(t2);
} catch (e) {
  console.log(e.constructor.name);
}
const str = "ab";
const [c1, c2] = str;
console.log(c1, c2);
const swap = { l: "L", r: "R" };
({ l: swap.r, r: swap.l } = swap);
console.log(swap.l, swap.r);
const { length } = "four";
console.log(length);
const [[n1, [n2]], { o: [n3] }] = [[1, [2]], { o: [3] }];
console.log(n1 + n2 + n3);
