const { s } = { s: { [Symbol.toPrimitive]() { return 1; } } };
const e = s + 1;
const g = 1 + s;
const t = s < 1;
const u = typeof (s - 1);
console.log(e, g, t, u);
