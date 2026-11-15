const it = { [Symbol.iterator]() { return [][Symbol.iterator](); } };
const e = it + 1;
const g = 1 + it;
const t = it > 0;
console.log(e, g, t);
