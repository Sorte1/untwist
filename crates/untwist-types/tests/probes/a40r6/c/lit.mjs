const o = { valueOf() { return 3; } };
const n = 2;
const m = n * 2;
const b = o < 1;
const c = o + 1;
const d = `${o}`;
console.log(m, b, c, d);
