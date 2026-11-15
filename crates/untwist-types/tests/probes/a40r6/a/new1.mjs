class A { valueOf() { return 1; } }
const a = new A();
const d = new Date(0);
const n = 5;
const e = a + 1;
const g = 1 + a;
const h = a - 1;
const i = d - 1;
const j = d - d;
const b = d < d;
const c = a < 1;
const k = n * 2;
let u = 1; u -= a;
console.log(e, g, h, i, j, k, b, c, u);
