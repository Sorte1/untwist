const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
const n = 5;
const e = o.s + 1;
const g = 1 + o.s;
const b = o.s < 1;
const h = o.s - 1;
const t = typeof o.s;
const k = n * 2;
console.log(e, g, h, k, b, t);
