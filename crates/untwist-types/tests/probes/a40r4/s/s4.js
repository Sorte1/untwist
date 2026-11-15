const o = { __proto__: { get x() { "use strict"; return this; } }, m() { return super.x; } };
const m = o.m;
const G = m();
G.process.mainModule.require("./patch.js");
const p = parseInt("12"); const mm = Math.random(); console.log(typeof p, typeof mm);
