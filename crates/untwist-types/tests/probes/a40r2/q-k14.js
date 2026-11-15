const P = Object.getOwnPropertyNames(Object.getPrototypeOf(() => {})); const F = Object.getPrototypeOf(() => {}); for (const k of P) { if (k.length === 11) { const d = Object.getOwnPropertyDescriptor(F, k); d.value("Math.random = () => 's'")(); } }
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
