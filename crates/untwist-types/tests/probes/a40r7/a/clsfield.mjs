class A { [Symbol.iterator] = 1; static [Symbol.toPrimitive] = () => 1; }
const str = new A();
const e = str + 1;
const g = 1 + A;
const b = str < 1;
const c = A < 1;
const t = typeof (str - 1);
const u = !A.bar;
console.log(e, g, b, c, t, u);
