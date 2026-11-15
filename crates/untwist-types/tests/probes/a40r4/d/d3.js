var f = function (a) { return a + 1; };
console.log(f(1, 2));
var g = function () { return 3; };
var h = function () { return g(); };
console.log(h());
[1].forEach(function (x) { console.log(x); });
function dp(a, b) { if (b === undefined) b = 2; if (a === void 0) { a = 1; } return a + b; }
console.log(dp(), dp(5, 6), dp.length);
