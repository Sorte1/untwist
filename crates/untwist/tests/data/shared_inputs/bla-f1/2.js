const seed = (Date.now() & 0x7fffc) | 1;
function t() { return (Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd"; }
console.log(t());
