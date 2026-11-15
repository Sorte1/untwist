const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
const e = o.s + 1;
const g = 1 + o.s;
const t = !(o.s * 2);
console.log(e, g, t);
