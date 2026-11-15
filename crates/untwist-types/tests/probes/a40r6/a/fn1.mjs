"use strict";
function f(o) { const n = 2; const m = n - 1; const b = o < 1; return [o + 1, 1 + o, o - 1, m, b]; }
function g() { const o = { [Symbol.toPrimitive]() { return 1; } }; const b = o > 1; const c = o + 1; const d = 1 + o; return [b, c, d]; }
console.log(f({ [Symbol.toPrimitive]() { return 1; } }), g());
