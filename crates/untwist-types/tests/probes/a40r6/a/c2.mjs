const str = { [Symbol.toPrimitive]() { return 1; } };
const e = str + 1;
const b = str < 1;
const g = 1 + str;
const c = str >= 2;
console.log(e, g, b, c);
