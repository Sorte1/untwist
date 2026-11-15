class A { [Symbol.iterator] = 1; }
const o = { s: { ...new A() } };
const e = o.s + 1;
const g = 1 + o.s;
const t = typeof (o.s - 1);
const u = `a${o.s * 2}`;
const v = !o.s.bar;
console.log(e, g, t, u, v);
