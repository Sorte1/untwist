function a() { var h = ['p', 'q', 'x', 'x']; a = function () { return h; }; return a(); }
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
var k = e[1];
while (!![]) {
try {
var f = b(2) === k ? 7 : 0;
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 7);
console.log(b(0), b(1), b(2), b(3));
