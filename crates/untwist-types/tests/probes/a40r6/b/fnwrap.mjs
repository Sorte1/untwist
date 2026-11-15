"use strict";
const o = { s: { [Symbol.toPrimitive]() { return 1; } } };
function f() { const e = o.s + 1; const g = 1 + o.s; const h = o.s - 1; return 1; }
const r = f();
console.log(r);
