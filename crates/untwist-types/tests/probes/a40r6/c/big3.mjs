function h(a, b) { const c = a + b; const d = a - b; const e = ~a; let k = a; k += b; return [typeof c, typeof d, typeof e, typeof k]; }
console.log(h(1, "2"), h(1n, "2"), h("x", 4));
