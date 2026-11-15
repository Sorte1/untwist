const str = class { static [Symbol.toPrimitive]() { return 1; } };
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof str.foo;
const u = !str.bar;
console.log(e, g, b, t, u);
