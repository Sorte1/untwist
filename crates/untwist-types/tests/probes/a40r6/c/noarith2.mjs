const it = { *[Symbol.iterator]() { yield 1; } };
const a = typeof it.size;
const b = typeof it.length;
const c = typeof it.count;
console.log(a, b, c, [...it].length);
