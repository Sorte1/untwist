const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
const e = o.s + 1;
const g = 1 + o.s;
const h = o.s - 1;
const n = 5;
const k = n - 1;
console.log(e, g, h, k);
