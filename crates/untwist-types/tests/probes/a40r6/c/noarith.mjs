const str = { [Symbol.toPrimitive]() { return 1; } };
const a = typeof str.foo;
const b = !str.bar;
const c = str.baz === 1;
const d = `${str.qux}`;
console.log(a, b, c, d);
