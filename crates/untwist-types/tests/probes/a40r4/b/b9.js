function f(a, b) { return ({ __proto__: f }).arguments.length; }
console.log(f(1, 2), f(1));
function h(x) { return ({ __proto__: h }).arguments[0]; }
function unusedArg() { return h(7); }
console.log(unusedArg());
function w() { return ({ __proto__: w }).caller.name; }
function outerName() { return w(); }
console.log(outerName());
