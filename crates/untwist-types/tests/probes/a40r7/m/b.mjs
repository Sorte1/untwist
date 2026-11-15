import { str, mk } from "./a.mjs";
const e = str + 1;
const g = 1 + str;
const b = str < 1;
const t = typeof (mk() - 1);
const u = `${str.qux}`;
console.log(e, g, b, t, u);
