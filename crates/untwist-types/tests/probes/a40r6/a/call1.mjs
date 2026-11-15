function mk() { return { [Symbol.toPrimitive]() { return 1; } }; }
const n = 5;
const e = mk() + 1;
const g = 1 + mk();
const h = mk() - 1;
const b = mk() < 1;
const k = n * 2;
console.log(e, g, h, k, b);
