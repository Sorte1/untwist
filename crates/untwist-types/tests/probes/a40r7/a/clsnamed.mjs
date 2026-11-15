class A { [Symbol.toPrimitive]() { return 1; } }
const str = new A();
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof (str - 1);
const u = !str.bar;
const v = `${str.qux}`;
console.log(e, g, b, t, u, v);
