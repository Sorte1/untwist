const a = { valueOf() { return 1; } };
const n = 5;
const e = a + 1;
const g = [1] + n;
const h = a + a;
let s = "x"; s += a;
let t = 1; t += a;
const k = n * 2;
console.log(e, g, h, s, t, k);
