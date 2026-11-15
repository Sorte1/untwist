const str = { [Math.random() < 2 ? Symbol.toPrimitive : Symbol.toPrimitive]() { return 1; } };
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof str.foo;
console.log(e, g, b, t);
