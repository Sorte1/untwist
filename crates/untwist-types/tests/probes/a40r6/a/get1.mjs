const o = { get s() { return { [Symbol.toPrimitive]() { return 1; } }; } };
const n = 5;
const e = o.s + 1;
const g = 1 + o.s;
const h = o.s - 1;
const b = o.s < 1;
const k = -n;
let u = 1; u *= o.s;
console.log(e, g, h, k, b, u);
