const big = { valueOf() { return 2n; } };
const str = { [Symbol.toPrimitive]() { return "s"; } };
const a = big * 2n; const b = -big; const c = ~big; let d = big; d++; const e = str + 1; const g = 1 + str; const h = str - 1;
const i = big ** 2n; const j = big >> 1n; const k = +str; let l = str; l--; let m = big; m += 1n; let n = str; n += 1; let q = 1; q += str;
let u = 1; u -= big - 1n;
console.log([a, b, c, d, e, g, h, i, j, k, l, m, n, q, u].map((x) => typeof x).join(" "));
