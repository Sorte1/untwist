var q = 1;
function has(o, k) { return Array.prototype.hasOwnProperty.call(o, k); }
console.log(has(this, "q"), has(window0(), "q"));
function window0() { return globalThis; }
