const seed = u() | 1;
function u() { try { console.log(t()); } catch (e) { console.log("caught " + e.name); } return 5; }
function t() { return (Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd"; }
console.log(t());
