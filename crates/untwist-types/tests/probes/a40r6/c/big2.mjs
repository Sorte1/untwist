function h(a, b) { const c = a + b; const d = a * b; const e = -a; const f = a < b; let i = a; i++; let j = b; j **= a; return [c, d, e, f, i, j]; }
console.log(h(1, 2), h(1n, 2n), h(3, 4));
