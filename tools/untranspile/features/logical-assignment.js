const o = { a: 0, b: 1, c: null, d: "x" };
o.a ||= 10;
o.b &&= 20;
o.c ??= 30;
o.d ??= "never";
console.log(JSON.stringify(o));
let x = null;
x ??= 5;
let y = 0;
y ||= 6;
let z = 1;
z &&= 7;
console.log(x, y, z);
let hits = 0;
const target = {
  get k() {
    hits++;
    return 1;
  },
  set k(v) {
    hits += 100;
  },
};
target.k ||= 2;
target.k &&= 3;
console.log(hits);
const arr = [null, 0];
let i = 0;
arr[i++] ??= "filled";
console.log(arr.join(","), i);
const deep = { l: { s: null, t: 0 }, m: { u: 1 } };
let keyCalls = 0;
const key = () => {
  keyCalls++;
  return "u";
};
(deep.l.s ??= []).push("pushed");
deep.l.t ||= "t";
deep.m[key()] &&= "u2";
const got = (deep.l.n ??= "made");
console.log(JSON.stringify(deep), keyCalls, got);
