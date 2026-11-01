function a() { var h = ['p', 'q', 'log', 'r', 's']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
var x = function (f, g) { b = y; return e[f - 0]; };
var y = function (f, g) { b = x; return e[f - 0] + '!'; };
b = x;
return b(c, d);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 0);
console.log(b(0), b(1), b(2), b(3), b(4));
