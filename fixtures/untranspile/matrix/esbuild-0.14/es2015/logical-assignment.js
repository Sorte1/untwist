var _a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k;
const o = { a: 0, b: 1, c: null, d: "x" };
o.a || (o.a = 10);
o.b && (o.b = 20);
(_a = o.c) != null ? _a : o.c = 30;
(_b = o.d) != null ? _b : o.d = "never";
console.log(JSON.stringify(o));
let x = null;
x != null ? x : x = 5;
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
(_d = arr[_c = i++]) != null ? _d : arr[_c] = "filled";
console.log(arr.join(","), i);
const deep = { l: { s: null, t: 0 }, m: { u: 1 } };
let keyCalls = 0;
const key = () => {
  keyCalls++;
  return "u";
};
((_f = (_e = deep.l).s) != null ? _f : _e.s = []).push("pushed");
(_g = deep.l).t || (_g.t = "t");
(_h = deep.m)[_i = key()] && (_h[_i] = "u2");
const got = (_k = (_j = deep.l).n) != null ? _k : _j.n = "made";
console.log(JSON.stringify(deep), keyCalls, got);
