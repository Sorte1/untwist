const str = { [Symbol.for("k")]() { return 1; } };
const e = str - 1;
const g = 1 - str;
const b = str < 1;
console.log(e, g, b);
