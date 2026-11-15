const a = new (class { [Symbol.toPrimitive]() { return 1; } })();
const e = a + 1;
const g = 1 + a;
const t = a < 1;
const u = typeof (a - 1);
console.log(e, g, t, u);
