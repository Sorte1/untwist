const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
const e = o.s + 1;
const g = 1 + o.s;
const h = o.s - 1;
const i = o.s * 2;
const n = 5;
const k = n * 2;
let m = 1; m++;
console.log(e, g, h, i, k, m);
