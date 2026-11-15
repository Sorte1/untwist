const o = { p: { [Symbol.toPrimitive]() { return 1; } }, q: [{ [Symbol.toPrimitive]() { return 1; } }] };
const n = 5;
const e = o.p + 1;
const g = 1 + o.p;
o.p++;
o.q[0]--;
const b = o.q < 1;
const k = n * 2;
console.log(e, g, k, b, typeof o.p);
