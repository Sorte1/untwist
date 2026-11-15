class Base { get exports() { return this; } }
const P = Base.prototype;
Object.defineProperty(Object.prototype, "exports", { get() { return this; }, configurable: true });
const G = exports;
const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p);
