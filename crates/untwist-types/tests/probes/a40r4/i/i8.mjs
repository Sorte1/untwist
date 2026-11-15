const d = { get() { return this; } };
const P = Object.prototype;
Object.defineProperty(P, "exports", d);
const p = parseInt("12"); const m = Math.random(); console.log(typeof p, typeof m);
