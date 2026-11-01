function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, d) {
var e = c();
var t = 0;
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (t++ > 11) break;
}
})(a, 0);
var a = function () { var h = ['p', 'q', 'log', 'r', 's']; a = function () { return h; }; return a(); };
console.log(b(0), b(1), b(2), b(3), b(4));
