const s = Symbol("x");
const n = 5;
const t = typeof s;
const b = s == n;
const c = !s;
const d = s === s;
const e = "p" in { [s]: 1 };
const k = n * 2;
console.log(t, b, c, d, e, k);
