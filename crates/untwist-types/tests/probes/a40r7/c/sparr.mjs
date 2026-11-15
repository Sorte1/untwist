const str = { ...[1, 2] };
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof str.foo;
console.log(e, g, b, t);
