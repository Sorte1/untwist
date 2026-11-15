const { toPrimitive } = Symbol;
const str = { [toPrimitive]() { return 1; } };
const e = str + 1;
const g = 1 + str;
const b = str < 1;
console.log(e, g, b);
