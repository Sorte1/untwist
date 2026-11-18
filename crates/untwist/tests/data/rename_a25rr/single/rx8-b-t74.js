function f(a) { return a + 1; }
function A(x) { switch (1) { case 1: var x = globalThis; } return x; } A({}).f = function () { return 99; };
console.log(f(1), f(2)); setTimeout(function () { console.log(f(1), f(2)); }, 5);
