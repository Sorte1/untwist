function a() { var h = ['p', 'q', 'log', 'r', 's']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
return (b = function (i, k) {
return (b = function (m, n) {
return e[m - 0] + '3';
}), e[i - 0] + '2';
}), e[f - 0];
}), b(c, d);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
var t = b(0);
if (t.charAt(0) === 'l') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 0);
console.log(b(0), b(1), b(2), b(3), b(4));
