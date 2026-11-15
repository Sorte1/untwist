const str = { [Symbol.toPrimitive]() { return 1; } };
const e = str + 1;
const g = 1 + str;
const h = str in str;
const i = str instanceof str;
console.log(e, g, h, i);
