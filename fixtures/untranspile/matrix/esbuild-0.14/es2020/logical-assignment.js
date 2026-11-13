var _a, _b, _c, _d, _e, _f;
const o = { a: 0, b: 1, c: null, d: "x" };
o.a || (o.a = 10);
o.b && (o.b = 20);
o.c ?? (o.c = 30);
o.d ?? (o.d = "never");
console.log(JSON.stringify(o));
let x = null;
x ?? (x = 5);
let y = 0;
y || (y = 6);
let z = 1;
z && (z = 7);
console.log(x, y, z);
let hits = 0;
const target = {
  get k() {
    hits++;
    return 1;
  },
  set k(v) {
    hits += 100;
  }
};
target.k || (target.k = 2);
target.k && (target.k = 3);
console.log(hits);
const arr = [null, 0];
let i = 0;
arr[_a = i++] ?? (arr[_a] = "filled");
console.log(arr.join(","), i);
const deep = { l: { s: null, t: 0 }, m: { u: 1 } };
let keyCalls = 0;
const key = () => {
  keyCalls++;
  return "u";
};
((_b = deep.l).s ?? (_b.s = [])).push("pushed");
(_c = deep.l).t || (_c.t = "t");
(_d = deep.m)[_e = key()] && (_d[_e] = "u2");
const got = (_f = deep.l).n ?? (_f.n = "made");
console.log(JSON.stringify(deep), keyCalls, got);
