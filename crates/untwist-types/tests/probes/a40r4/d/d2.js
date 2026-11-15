var f = function (a) { return f.arguments.length; };
console.log(f(1, 2));
var g = function () { return g.caller === h; };
var h = function () { return g(); };
console.log(h());
function dp(a, b) { if (b === undefined) b = 2; if (a === void 0) { a = 1; } return a + b; }
console.log(dp(), dp(5, 6), dp.length);
