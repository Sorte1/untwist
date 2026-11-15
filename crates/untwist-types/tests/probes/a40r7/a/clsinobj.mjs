const o = { s: new (class { [Symbol.toPrimitive]() { return 1; } })(), k: class { static [Symbol.iterator] = 1 } };
const e = o.s + 1;
const g = 1 + o.k;
const b = o.s < 1;
const t = typeof (o.k - 1);
const u = !o.s.bar;
console.log(e, g, b, t, u);
