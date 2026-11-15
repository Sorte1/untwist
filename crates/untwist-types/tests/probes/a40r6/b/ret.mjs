const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
const e = o.s + 1;
const g = 1 + o.s;
function f() { return o.s < 1; }
function h() { return typeof (o.s - 1); }
console.log(e, g, f(), h());
