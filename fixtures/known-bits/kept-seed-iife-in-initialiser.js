const seed = (function () { try { console.log((Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd"); } catch (e) { console.log("caught " + e.name); } return 4; })() | 1;
console.log((Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd");
