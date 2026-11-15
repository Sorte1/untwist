const str = { get [S().toPrimitive]() { return () => 1; } };
function S() { return Symbol; }
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof str.foo;
console.log(e, g, b, t);
