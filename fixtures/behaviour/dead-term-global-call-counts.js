function a() { var h = ['x', 'y', '3', '4', '5']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
var n = 0;
function g() { n = n + 1; return 0; }
(function (c, d) {
var e = c();
while (!![]) {
try {
var f = n + parseInt(b(0)) + g();
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 5);
console.log(b(0), b(1), b(2), b(3), b(4), n);
