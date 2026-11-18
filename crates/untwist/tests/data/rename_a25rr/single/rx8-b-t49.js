function f(a) { return a + 1; }
function A(x) { var [x] = [globalThis]; return x; } A({}).f = function () { return 99; };
console.log(f(1), f(2)); setTimeout(function () { console.log(f(1), f(2)); }, 5);
