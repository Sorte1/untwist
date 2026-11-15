let S2; const str = { [(S2 = Symbol).toPrimitive]() { return 1; } };
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof str.foo;
console.log(e, g, b, t);
