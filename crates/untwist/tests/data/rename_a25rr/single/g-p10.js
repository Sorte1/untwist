function q(o) { return o; }
console.log(q(1), "fn" in globalThis, "a" in globalThis, "fn1" in globalThis, "f" in globalThis);
