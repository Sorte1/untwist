function inner(a, b) { return a - b; }
function wrap(a, b) { return inner(a, b); }
console.log(wrap(5, 3));
