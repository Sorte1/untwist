const D = [3];
function f(n) { D.push(1); return D.length - 1; }
this.f(0);
const seed = (typeof Date !== "undefined" ? Date.now() & 0x7fffc : 4) & 0x7fffc | (typeof f === "function" && f(0) === 1 ? 1 : 0);
console.log((Math.imul(seed ^ 8, seed ^ 8) & 7) === 1 ? "match" : "mismatch");
