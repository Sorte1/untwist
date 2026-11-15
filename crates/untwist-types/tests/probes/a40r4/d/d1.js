var f = function (a) { return ({ __proto__: f }).arguments.length; };
console.log(f(1, 2));
var g = function () { return ({ __proto__: g }).caller === h; };
var h = function () { return g(); };
console.log(h());
[1].forEach(function cb(x) { console.log(({ __proto__: cb }).arguments.length); });
function dp(a, b) { if (b === undefined) b = 2; if (a === void 0) { a = 1; } return ({ __proto__: dp }).arguments.length + a + b; }
console.log(dp(), dp(5, 6), dp.length);
var k = function (x) { var arguments = k; return arguments.arguments.length; };
console.log(k(1, 2, 3));
