function f(a) { const b = a - 1; const c = -a; let d = a; d++; return [b, c, d]; }
function g(a, b) { const c = a + b; const d = a * b; return [c, d]; }
console.log(typeof f, g(1, 2), g(1n, 2n));
