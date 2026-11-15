function h(a) { const t = typeof a; const d = a === a; const e = !a; const s = `${typeof a}`; return [t, d, e, s]; }
console.log(h(Symbol("x")), h(1));
const s = Symbol.iterator;
const u = typeof s;
const v = s == null;
let w = 1; w += 2; w -= 1;
console.log(u, v, w);
