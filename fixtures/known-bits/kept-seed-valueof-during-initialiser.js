var h = { valueOf() { try { console.log(t()); } catch (e) { console.log("caught " + e.name); } return 4; } };
const seed = h | 1;
function t() { return (Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd"; }
console.log(t());
