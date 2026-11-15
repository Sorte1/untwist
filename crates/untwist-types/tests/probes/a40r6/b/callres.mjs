function mk() { return { [Symbol.toPrimitive]() { return 1; } }; }
const e = mk() + 1;
const g = 1 + mk();
const u = typeof (mk() - 1);
console.log(e, g, u);
