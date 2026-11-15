const o = { get s() { return { [Symbol.toPrimitive]() { return 1; } }; } };
const e = o.s + 1;
const g = 1 + o.s;
const u = !(o.s - 1);
console.log(e, g, u);
