function a() { var h = ['x', '3', '3a', 'z', 'w']; a = function () { return h; }; return a(); }
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
var o = { get v() { e['push'](e['shift']()); return 0; } };
while (!![]) {
try {
var f = parseInt(b(0)) + o.v;
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 3);
console.log(b(0), b(1), b(2), b(3), b(4));
