function a() { var h = ['x', '5', '5', 'q']; a = function () { return h; }; return a(); }
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
while (!![]) {
try {
var f = parseInt(b(0)) + (b(2) === 'q' ? '' : 0);
if (f == d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 5);
console.log(b(0), b(1), b(2), b(3));
